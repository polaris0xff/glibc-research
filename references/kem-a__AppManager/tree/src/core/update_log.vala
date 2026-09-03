namespace AppManager.Core {
    public class UpdateLog {
        private const int64 MAX_SIZE = 1048576;
        private const int KEEP = 3;

        public static void append(string message) {
            DirUtils.create_with_parents(AppPaths.data_dir, 0755);
            var path = AppPaths.updates_log_file;
            rotate_if_needed(path);
            var ts = new GLib.DateTime.now_local().format("%FT%T%z");
            var line = "%s %s\n".printf(ts, message);
            var file = FileStream.open(path, "a");
            if (file != null) {
                file.puts(line);
                file.flush();
            }
        }

        private static void rotate_if_needed(string path) {
            Posix.Stat st;
            if (Posix.stat(path, out st) != 0) return;
            if (st.st_size < MAX_SIZE) return;

            FileUtils.unlink("%s.%d".printf(path, KEEP));
            for (int i = KEEP - 1; i >= 1; i--) {
                FileUtils.rename("%s.%d".printf(path, i), "%s.%d".printf(path, i + 1));
            }
            FileUtils.rename(path, "%s.1".printf(path));
        }
    }
}
