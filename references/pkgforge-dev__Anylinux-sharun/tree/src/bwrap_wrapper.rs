// bwrap-wrapper — intercepts bubblewrap to inject essential bind mounts
// and remap hardcoded paths inside the sandbox.
//
// Many applications sandbox themselves via bwrap (e.g. WebKitGTK) but don't
// account for AppImage paths. Symlinks created by sharun in /tmp become
// unresolvable inside the sandbox. This wrapper injects:
//   --bind $APPDIR $APPDIR   so the AppImage mount is visible inside
//   --bind /tmp /tmp         so sharun's /tmp symlinks stay valid
//   --setenv SHARUN_DIR ...  so child processes know the symlink prefix
//   --setenv APPDIR ...      so the AppDir path survives into the sandbox
//   --setenv PATH ...        so binaries in $APPDIR/bin get executed always
//   --proc /proc             so that sharun can read /proc/self/exe and work
//
// It also rewrites hardcoded command paths (e.g. /usr/bin/xdg-dbus-proxy) to
// their AppDir equivalents when found, so the AppImage's bundled binaries are
// used instead of the host.
//
// --seccomp is stripped because it blocks lstat, which sharun needs to resolve
// /proc/self/exe for binaries running inside the sandbox.
//
// Two codepaths: "--args N" (options passed through a pipe, used by WebKitGTK)
// and plain argv.

use std::os::unix::io::{FromRawFd, IntoRawFd, RawFd};
use std::path::Path;

use nix::unistd::{access, pipe, AccessFlags};

fn opt_arg_count(arg: &str) -> usize {
	match arg {
		"--overlay" => 3,
		"--bind" | "--ro-bind" | "--bind-try" | "--ro-bind-try" |
		"--dev-bind" | "--dev-bind-try" | "--bind-data" | "--ro-bind-data" |
		"--file" | "--ro-file" | "--dev-mknod" | "--symlink" |
		"--chmod" | "--bind-fd" | "--ro-bind-fd" | "--setenv" => 2,
		"--tmpfs" | "--proc" | "--dev" | "--devpts" | "--mqueue" |
		"--hostname" | "--seccomp" | "--block-fd" | "--userns" |
		"--uid" | "--gid" | "--chdir" | "--unsetenv" | "--lock-file" |
		"--sync-fd" | "--info-fd" | "--json-status-fd" | "--add-seccomp-fd" |
		"--add-feature" | "--args" | "--dir" | "--remount-ro" |
		"--perms" | "--size" | "--argv0" | "--overlay-src" |
		"--tmp-overlay" | "--ro-overlay" | "--exec-label" | "--file-label" |
		"--userns-block-fd" | "--pidns" => 1,
		_ => 0,
	}
}

fn find_cmd_idx(args: &[String]) -> usize {
	let mut i = 0;
	while i < args.len() {
		if args[i] == "--" {
			return i;
		}
		if args[i].starts_with('-') {
			i += 1 + opt_arg_count(&args[i]);
			continue;
		}
		return i;
	}
	args.len()
}

fn build_injections(appdir: &str, sharun_dir: &str, path: &str) -> Vec<String> {
	let mut inj = Vec::new();

	inj.push("--proc".into());
	inj.push("/proc".into());

	if !appdir.is_empty() {
		inj.push("--bind".into());
		inj.push(appdir.into());
		inj.push(appdir.into());
	}

	inj.push("--bind".into());
	inj.push("/tmp".into());
	inj.push("/tmp".into());

	if !sharun_dir.is_empty() {
		inj.push("--setenv".into());
		inj.push("SHARUN_DIR".into());
		inj.push(sharun_dir.into());
	}

	if !appdir.is_empty() {
		inj.push("--setenv".into());
		inj.push("APPDIR".into());
		inj.push(appdir.into());
	}

	if !path.is_empty() {
		inj.push("--setenv".into());
		inj.push("PATH".into());
		inj.push(path.into());
	}

	inj
}

fn try_remap_path(path: &str, appdir: &str) -> Option<String> {
	if !path.starts_with('/') || appdir.is_empty() {
		return None;
	}
	let base = path.rsplit('/').next()?;
	if base.is_empty() {
		return None;
	}
	for dir in &["bin", "lib", "libexec"] {
		let candidate = format!("{appdir}/{dir}/{base}");
		if access(candidate.as_str(), AccessFlags::X_OK).is_ok() {
			return Some(candidate);
		}
	}
	None
}

fn remap_command(args: &mut Vec<String>, appdir: &str) {
	if appdir.is_empty() {
		return;
	}
	let mut i = 0;
	while i < args.len() {
		if args[i] == "--" {
			if i + 1 < args.len() {
				if let Some(r) = try_remap_path(&args[i + 1], appdir) {
					args[i + 1] = r;
				}
			}
			return;
		}
		if args[i].starts_with('-') {
			i += 1 + opt_arg_count(&args[i]);
			continue;
		}
		if let Some(r) = try_remap_path(&args[i], appdir) {
			args[i] = r;
		}
		return;
	}
}

fn read_fd_all(fd: RawFd) -> Vec<u8> {
	use std::io::Read;
	let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
	let mut buf = Vec::new();
	let _ = file.read_to_end(&mut buf);
	buf
}

fn serialize_to_pipe(args: &[String]) -> Option<RawFd> {
	use std::io::Write;

	let mut buf = Vec::new();
	for arg in args {
		buf.extend_from_slice(arg.as_bytes());
		buf.push(0);
	}

	let (read_fd, write_fd) = pipe().ok()?;

	let read_raw = read_fd.into_raw_fd();
	let write_raw = write_fd.into_raw_fd();

	let mut writer = unsafe { std::fs::File::from_raw_fd(write_raw) };
	if writer.write_all(&buf).is_err() {
		eprintln!("bwrap-wrapper: failed to write to pipe");
		std::process::exit(1);
	}
	drop(writer);

	Some(read_raw)
}

fn process_args_pipe(
	exec_args: &[String],
	args_idx: usize,
	args_fd: RawFd,
	injections: &[String],
	appdir: &str,
) -> (Vec<String>, Option<RawFd>) {
	let content = read_fd_all(args_fd);

	let fd_args: Vec<String> = content
		.split(|&b| b == 0)
		.filter(|s| !s.is_empty())
		.map(|s| String::from_utf8_lossy(s).into_owned())
		.collect();

	let cmd_idx = find_cmd_idx(&fd_args);

	let mut pipe_opts: Vec<String> = Vec::new();
	{
		let mut i = 0;
		while i < cmd_idx {
			if fd_args[i] == "--seccomp" {
				i += 2;
				continue;
			}
			pipe_opts.push(fd_args[i].clone());
			i += 1;
		}
	}
	pipe_opts.extend_from_slice(injections);

	let new_fd = serialize_to_pipe(&pipe_opts).unwrap_or_else(|| {
		eprintln!("bwrap-wrapper: failed to create pipe");
		std::process::exit(1);
	});

	let fd_str = new_fd.to_string();
	let mut new_args: Vec<String> = Vec::new();

	for (i, arg) in exec_args.iter().enumerate() {
		if i == args_idx {
			new_args.push("--args".into());
			new_args.push(fd_str.clone());
		} else if i == args_idx + 1 {
			continue;
		} else {
			new_args.push(arg.clone());
		}
	}

	new_args.extend(fd_args[cmd_idx..].iter().cloned());

	remap_command(&mut new_args, appdir);

	(new_args, Some(new_fd))
}

fn process_args_direct(
	exec_args: &[String],
	injections: &[String],
	appdir: &str,
) -> Vec<String> {
	let mut stripped: Vec<String> = Vec::with_capacity(exec_args.len());
	{
		let mut i = 0;
		while i < exec_args.len() {
			if exec_args[i] == "--seccomp" {
				i += 2;
				continue;
			}
			stripped.push(exec_args[i].clone());
			i += 1;
		}
	}

	let insert_at = find_cmd_idx(&stripped);

	let mut new_args: Vec<String> = Vec::with_capacity(stripped.len() + injections.len());
	for (i, arg) in stripped.iter().enumerate() {
		if i == insert_at {
			for inj in injections {
				new_args.push(inj.clone());
			}
		}
		new_args.push(arg.clone());
	}
	if insert_at >= stripped.len() {
		for inj in injections {
			new_args.push(inj.clone());
		}
	}

	remap_command(&mut new_args, appdir);

	new_args
}

pub fn process_bwrap_args(
	exec_args: Vec<String>,
	appdir: &str,
	sharun_dir: &str,
	path: &str,
) -> (Vec<String>, Option<RawFd>) {
	let injections = build_injections(appdir, sharun_dir, path);

	for (i, arg) in exec_args.iter().enumerate() {
		if arg == "--args" && i + 1 < exec_args.len() {
			if let Ok(fd) = exec_args[i + 1].parse::<RawFd>() {
				return process_args_pipe(&exec_args, i, fd, &injections, appdir);
			}
		}
	}

	(process_args_direct(&exec_args, &injections, appdir), None)
}

pub fn find_system_bwrap(search_path: &str, sharun: &Path) -> Option<String> {
	for dir in search_path.split(':') {
		if dir.is_empty() {
			continue;
		}
		let candidate = format!("{dir}/bwrap");
		let candidate_path = Path::new(&candidate);
		if crate::utils::is_exe(candidate_path) &&
			!crate::utils::is_hardlink(sharun, candidate_path) {
			return Some(candidate);
		}
	}
	None
}
