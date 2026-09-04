//! Reads an RPM repository's repodata — `repomd.xml` names where
//! `primary.xml` (packages) and `filelists.xml` (file ownership) live —
//! into the package index and the path index. Every piece is fetched
//! from the mirror that served the `repomd.xml`, since the checksummed
//! paths are mirror-specific.

use std::sync::Arc;

use anyhow::{Context, Result, bail};

use crate::db::IndexWriter;
use crate::distro::Repository;
use crate::internal::codec::Codec;
use crate::internal::http::HttpClient;
use crate::package::{DepSpec, Dependency, Package, RichDep};
use crate::pkg::rpm::isa::IsaQualifier;
use crate::pkg::rpm::richdep::RichDepExpr;

/// Reads every repository of one source into the package index,
/// carrying the writer, HTTP client, and target architecture across
/// repositories.
pub(super) struct RepodataReader<'a, 'w> {
  writer: &'a mut IndexWriter<'w>,
  http: &'a Arc<HttpClient>,
  arch_target: &'a str,
}

impl<'a, 'w> RepodataReader<'a, 'w> {
  /// A reader that records into `writer`, fetches over `http`, and
  /// drops packages built for architectures other than `arch_target`.
  pub(super) fn new(writer: &'a mut IndexWriter<'w>, http: &'a Arc<HttpClient>, arch_target: &'a str) -> Self {
    RepodataReader {
      writer,
      http,
      arch_target,
    }
  }

  /// Reads one repository in full: resolves the metadata paths through
  /// `repomd.xml`, pins every further fetch to the mirror that served
  /// it so checksums match, and writes the packages and file lists into
  /// the index.
  pub(super) fn repo_ingest(&mut self, repo: &Repository) -> Result<()> {
    use crate::mirror::{HttpMirror, Mirror};

    let path_dir_repo = match &repo.layout {
      crate::distro::LayoutRepository::Rpm { path_dir_repo } => path_dir_repo,
      other => bail!("rpm index_fetch received non-Rpm layout for repository {}: {:?}", repo.label, other),
    };

    let path_repomd = format!("{path_dir_repo}/repodata/repomd.xml");
    let (bytes_repomd, mirror_origin) = repo
      .mirrors
      .fetch_with_origin(&path_repomd)
      .with_context(|| format!("repomd.xml for {}", repo.label))?;
    let text_repomd = String::from_utf8_lossy(&bytes_repomd).to_string();

    let primary_href = RepodataReader::repomd_href(&text_repomd, "primary")
      .with_context(|| format!("primary.xml href in {}", repo.label))?;
    let filelists_href = RepodataReader::repomd_href(&text_repomd, "filelists")
      .with_context(|| format!("filelists.xml href in {}", repo.label))?;

    // Pin to the mirror that delivered the repomd — primary.xml and
    // filelists.xml carry mirror-specific content checksums, so a
    // fallthrough between repomd and the metadata it names would
    // produce mismatched digests.
    let mirror_pinned = HttpMirror::live(mirror_origin.clone(), self.http.clone());
    let path_primary = format!("{path_dir_repo}/{primary_href}");
    let path_filelists = format!("{path_dir_repo}/{filelists_href}");

    let bytes_primary = mirror_pinned
      .fetch(&path_primary)
      .with_context(|| format!("primary.xml for {}", repo.label))?;
    let bytes_filelists = mirror_pinned
      .fetch(&path_filelists)
      .with_context(|| format!("filelists.xml for {}", repo.label))?;

    let text_primary = String::from_utf8(Codec::from_suffix(&primary_href).bytes(&bytes_primary)?)
      .with_context(|| format!("primary metadata is not valid UTF-8 for {}", repo.label))?;
    let reader_filelists = Codec::from_suffix(&filelists_href).reader(&bytes_filelists)?;

    let url_repo = format!("{}/{}", mirror_origin.trim_end_matches('/'), path_dir_repo);
    let packages = RepodataReader::primary_parse(&text_primary, self.arch_target, &url_repo)?;
    self
      .writer
      .packages_insert(packages, &crate::version::RpmVersionCompare)?;
    self.writer.paths_ingest(reader_filelists)?;
    Ok(())
  }

  /// The `href` of one `<data type=...>` entry in `repomd.xml`. The
  /// path embeds a checksum and changes on every regeneration, so it
  /// must be read, never guessed.
  fn repomd_href(xml: &str, data_type: &str) -> Result<String> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(xml);
    let mut in_target = false;
    let data_type_bytes = data_type.as_bytes();

    loop {
      match reader.read_event() {
        Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
          let local = e.local_name();
          if local.as_ref() == b"data" {
            in_target = e
              .attributes()
              .filter_map(|a| a.ok())
              .any(|a| a.key.local_name().as_ref() == b"type" && a.value.as_ref() == data_type_bytes);
          } else if in_target && local.as_ref() == b"location" {
            for attr in e.attributes().filter_map(|a| a.ok()) {
              if attr.key.local_name().as_ref() == b"href" {
                return Ok(String::from_utf8_lossy(&attr.value).to_string());
              }
            }
          }
        }
        Ok(Event::End(ref e)) if e.local_name().as_ref() == b"data" => {
          in_target = false;
        }
        Ok(Event::Eof) => break,
        Err(e) => bail!("XML parse error in repomd.xml: {}", e),
        _ => {}
      }
    }

    bail!("No <data type=\"{}\"> found in repomd.xml", data_type)
  }

  /// Parses `primary.xml` into `Package` records: other-architecture
  /// packages are dropped, each record carries its download location,
  /// conditional dependencies are kept as `RichDep`s, and version
  /// operators are re-spelled in the dpkg form.
  fn primary_parse(xml: &str, target_arch: &str, url_prefix: &str) -> Result<Vec<Package>> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let mut reader = Reader::from_str(xml);
    let mut parser = PrimaryParser::new(target_arch, url_prefix);

    loop {
      match reader.read_event() {
        Ok(Event::Start(ref e)) => parser.start_handle(e),
        Ok(Event::Empty(ref e)) => parser.empty_handle(e)?,
        Ok(Event::Text(ref e)) => parser.text_handle(e),
        Ok(Event::End(ref e)) => parser.end_handle(e),
        Ok(Event::Eof) => break,
        Err(e) => bail!("XML parse error in primary.xml: {}", e),
        _ => {}
      }
    }

    Ok(parser.packages)
  }
}

/// Which `<format>` relationship section the cursor currently sits inside.
#[derive(PartialEq)]
enum Section {
  None,
  Provides,
  Requires,
  Conflicts,
  Obsoletes,
}

/// Streaming state for one pass over primary.xml: which structural region the
/// cursor is inside, the half-gathered facts of the package currently open,
/// and the finished records. One package's facts become a `Package` only when
/// its closing tag arrives and its architecture qualifies.
struct PrimaryParser<'a> {
  target_arch: &'a str,
  url_prefix: &'a str,
  packages: Vec<Package>,
  in_package: bool,
  in_format: bool,
  section: Section,
  current_tag: String,
  draft: PackageDraft,
}

/// The per-package accumulator: every fact primary.xml states about the
/// package currently open, gathered until its closing tag either turns the
/// draft into a `Package` or discards it as foreign-architecture.
#[derive(Default)]
struct PackageDraft {
  name: String,
  arch: String,
  version: String,
  checksum: String,
  summary: String,
  size: u64,
  location: String,
  provides: Vec<DepSpec>,
  requires: Vec<Dependency>,
  conflicts: Vec<Dependency>,
  obsoletes: Vec<Dependency>,
  rich_deps: Vec<RichDep>,
}

impl<'a> PrimaryParser<'a> {
  fn new(target_arch: &'a str, url_prefix: &'a str) -> Self {
    PrimaryParser {
      target_arch,
      url_prefix,
      packages: Vec::new(),
      in_package: false,
      in_format: false,
      section: Section::None,
      current_tag: String::new(),
      draft: PackageDraft::default(),
    }
  }

  fn start_handle(&mut self, e: &quick_xml::events::BytesStart) {
    let ln = e.local_name();
    let local = std::str::from_utf8(ln.as_ref()).unwrap_or("");
    match local {
      "package" => {
        self.in_package = true;
        self.draft = PackageDraft::default();
      }
      "format" => self.in_format = true,
      "provides" if self.in_format => self.section = Section::Provides,
      "requires" if self.in_format => self.section = Section::Requires,
      "conflicts" if self.in_format => self.section = Section::Conflicts,
      "obsoletes" if self.in_format => self.section = Section::Obsoletes,
      _ => {}
    }
    if !self.in_format {
      self.current_tag = local.to_string();
    }
  }

  fn empty_handle(&mut self, e: &quick_xml::events::BytesStart) -> Result<()> {
    if !self.in_package {
      return Ok(());
    }
    let ln = e.local_name();
    let local = std::str::from_utf8(ln.as_ref()).unwrap_or("");
    match local {
      "version" if !self.in_format => self.draft.version = Self::version_read(e),
      "size" => self.draft.size = Self::size_read(e)?,
      "location" => {
        if let Some(href) = Self::attr_get(e, "href") {
          self.draft.location = format!("{}|{}", self.url_prefix, href);
        }
      }
      "entry" if self.in_format => self.entry_record(EntryAttrs::read(e))?,
      _ => {}
    }
    Ok(())
  }

  fn text_handle(&mut self, e: &quick_xml::events::BytesText) {
    if !self.in_package || self.in_format {
      return;
    }
    let text = e.unescape().unwrap_or_default();
    match self.current_tag.as_str() {
      "name" => self.draft.name = text.to_string(),
      "arch" => self.draft.arch = text.to_string(),
      "summary" => self.draft.summary = text.to_string(),
      "checksum" => self.draft.checksum = text.to_string(),
      _ => {}
    }
  }

  fn end_handle(&mut self, e: &quick_xml::events::BytesEnd) {
    let ln = e.local_name();
    let local = std::str::from_utf8(ln.as_ref()).unwrap_or("");
    match local {
      "package" if self.in_package => {
        self.in_package = false;
        if let Some(pkg) = self.draft_finish() {
          self.packages.push(pkg);
        }
      }
      "format" => {
        self.in_format = false;
        self.section = Section::None;
      }
      "provides" | "requires" | "conflicts" | "obsoletes" if self.in_format => {
        self.section = Section::None;
      }
      _ => {}
    }
    if !self.in_format {
      self.current_tag.clear();
    }
  }

  /// Routes one decoded `<entry>` into the bucket its enclosing section owns.
  fn entry_record(&mut self, entry: EntryAttrs) -> Result<()> {
    if entry.name.is_empty() {
      return Ok(());
    }
    match self.section {
      Section::Provides => self.draft.provides.push(entry.into_provide()),
      Section::Requires => self.require_record(entry)?,
      Section::Conflicts => self.draft.conflicts.push(entry.into_bare_dependency()),
      Section::Obsoletes => self.draft.obsoletes.push(entry.into_bare_dependency()),
      Section::None => {}
    }
    Ok(())
  }

  /// A requires entry is one of three things: packaging-tool noise to drop, a
  /// rich-dependency formula to parse (held back whole when conditional,
  /// flattened when not), or a plain requirement.
  fn require_record(&mut self, entry: EntryAttrs) -> Result<()> {
    if entry.is_build_only() {
      return Ok(());
    }
    if !entry.name.starts_with('(') {
      self.draft.requires.push(entry.into_versioned_dependency());
      return Ok(());
    }
    let ast = RichDepExpr::parse(&entry.name)
      .with_context(|| format!("Failed to parse rich dep for package '{}'", self.draft.name))?;
    if ast.contains_conditional() {
      self.draft.rich_deps.push(ast);
      return Ok(());
    }
    self.draft.requires.extend(
      RichDepExpr::flatten(&ast)
        .with_context(|| format!("Failed to flatten rich dep for package '{}'", self.draft.name))?,
    );
    Ok(())
  }

  /// A closed package block becomes a record only when it is installable here:
  /// built for the target architecture (or architecture-neutral) and carrying
  /// a name. Anything else is discarded with the draft.
  fn draft_finish(&mut self) -> Option<Package> {
    let draft = std::mem::take(&mut self.draft);
    let installable = (draft.arch == self.target_arch || draft.arch == "noarch") && !draft.name.is_empty();
    if !installable {
      return None;
    }
    Some(Package {
      name: draft.name,
      version: draft.version,
      depends: draft.requires,
      provides: draft.provides,
      recommends: Vec::new(),
      suggests: Vec::new(),
      install_if: Vec::new(),
      conflicts: draft.conflicts,
      breaks: draft.obsoletes,
      essential: false,
      priority: None,
      description: draft.summary,
      filename: draft.location,
      size: draft.size,
      checksum: draft.checksum,
      rich_deps: draft.rich_deps,
    })
  }

  /// `<version epoch=.. ver=.. rel=../>` → one flat version string,
  /// with a non-zero epoch carried as the `epoch:` prefix.
  fn version_read(e: &quick_xml::events::BytesStart) -> String {
    let epoch = Self::attr_get(e, "epoch").unwrap_or_else(|| String::from("0"));
    let ver = Self::attr_get(e, "ver").unwrap_or_default();
    let rel = Self::attr_get(e, "rel").unwrap_or_default();
    if epoch != "0" {
      format!("{}:{}-{}", epoch, ver, rel)
    } else {
      format!("{}-{}", ver, rel)
    }
  }

  /// `<size package=.../>` — the archive byte count, refused with context when
  /// the attribute is not a number.
  fn size_read(e: &quick_xml::events::BytesStart) -> Result<u64> {
    let Some(raw) = Self::attr_raw_get(e, "package") else {
      return Ok(0);
    };
    let raw = std::str::from_utf8(&raw).context("primary.xml <size package=...> is not valid UTF-8")?;
    raw
      .parse()
      .with_context(|| format!("primary.xml <size package=...> '{raw}' is not an integer"))
  }

  fn attr_get(e: &quick_xml::events::BytesStart, name: &str) -> Option<String> {
    Self::attr_raw_get(e, name).map(|v| String::from_utf8_lossy(&v).to_string())
  }

  fn attr_raw_get(e: &quick_xml::events::BytesStart, name: &str) -> Option<Vec<u8>> {
    e.attributes()
      .filter_map(|a| a.ok())
      .find(|a| a.key.local_name().as_ref() == name.as_bytes())
      .map(|a| a.value.into_owned())
  }
}

/// One `<entry>` under provides/requires/conflicts/obsoletes, with the four
/// attributes record-building cares about already decoded, plus the renderings
/// each section needs.
#[derive(Default)]
struct EntryAttrs {
  name: String,
  flags: String,
  ver: String,
  rel: String,
}

impl EntryAttrs {
  fn read(e: &quick_xml::events::BytesStart) -> EntryAttrs {
    let mut entry = EntryAttrs::default();
    for attr in e.attributes().filter_map(|a| a.ok()) {
      let kln = attr.key.local_name();
      match std::str::from_utf8(kln.as_ref()).unwrap_or("") {
        "name" => {
          entry.name = attr
            .unescape_value()
            .unwrap_or_else(|_| String::from_utf8_lossy(&attr.value).into())
            .to_string();
        }
        "flags" => entry.flags = String::from_utf8_lossy(&attr.value).to_string(),
        "ver" => entry.ver = String::from_utf8_lossy(&attr.value).to_string(),
        "rel" => entry.rel = String::from_utf8_lossy(&attr.value).to_string(),
        _ => {}
      }
    }
    entry
  }

  /// rpmlib(...)/config(...)/rtld(...) entries describe the packaging tool's
  /// own capabilities, not installable packages.
  fn is_build_only(&self) -> bool {
    self.name.starts_with("rpmlib(")
      || self.name.starts_with("config(")
      || self.name.starts_with("rtld(")
      || self.name == "this-is-only-for-build-envs"
  }

  /// Version-release as primary.xml spells it: `ver` alone, or `ver-rel`.
  fn version_release(&self) -> Option<String> {
    if self.ver.is_empty() {
      return None;
    }
    if self.rel.is_empty() {
      return Some(self.ver.clone());
    }
    Some(format!("{}-{}", self.ver, self.rel))
  }

  /// The provides version is a plain version string (not a constraint); the
  /// resolver checks dep constraints against it.
  fn into_provide(self) -> DepSpec {
    let version = self.version_release();
    DepSpec {
      name: IsaQualifier::arch_strip(&self.name),
      version_constraint: version,
    }
  }

  fn into_bare_dependency(self) -> Dependency {
    Dependency {
      alternatives: vec![DepSpec {
        name: IsaQualifier::arch_strip(&self.name),
        version_constraint: None,
      }],
    }
  }

  fn into_versioned_dependency(self) -> Dependency {
    let constraint = self.constraint_render();
    Dependency {
      alternatives: vec![DepSpec {
        name: IsaQualifier::arch_strip(&self.name),
        version_constraint: constraint,
      }],
    }
  }

  /// RPM spells comparison flags as words; the constraint is re-spelled in the
  /// dpkg form the comparator vocabulary uses (`<`/`>` become `<<`/`>>`).
  fn constraint_render(&self) -> Option<String> {
    if self.flags.is_empty() {
      return None;
    }
    let version = self.version_release()?;
    let op = match self.flags.as_str() {
      "EQ" => "=",
      "GE" => ">=",
      "GT" => ">>",
      "LE" => "<=",
      "LT" => "<<",
      _ => ">=",
    };
    Some(format!("{} {}", op, version))
  }
}
