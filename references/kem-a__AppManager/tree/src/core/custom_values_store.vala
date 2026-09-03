namespace AppManager.Core {
    /**
     * Manages custom user values (custom_commandline_args, custom_update_link, etc.)
     * in a separate custom.json file, decoupled from the main installations.json.
     *
     * This allows users to easily migrate or backup their customizations independently
     * of the installation metadata.
     *
     * Entries are keyed by lowercase app name. When an app is uninstalled, its custom
     * values remain in custom.json for automatic restoration on reinstall.
     */
    public class CustomValuesStore : Object {
        private HashTable<string, Json.Object> entries;
        private File store_file;
        private Mutex store_mutex = Mutex();

        public CustomValuesStore() {
            entries = new HashTable<string, Json.Object>(GLib.str_hash, GLib.str_equal);
            store_file = File.new_for_path(AppPaths.custom_values_file);
            load();
        }

        /**
         * Applies custom values from the store to a record, matched by name.
         * Only sets values that are currently null on the record
         * (doesn't overwrite values already set, e.g. carried over during upgrade).
         */
        public void apply_to_record(InstallationRecord record) {
            store_mutex.lock();
            var obj = entries.get(record.name.down());
            store_mutex.unlock();
            if (obj != null) {
                record.apply_history(obj);
                debug("Applied custom values from store for %s", record.name);
            }
        }

        /**
         * Saves custom values from a record into the store.
         * If the record has no custom values, removes the entry.
         */
        public void save_from_record(InstallationRecord record) {
            store_mutex.lock();
            if (record.has_custom_values()) {
                var node = record.to_custom_values_json();
                entries.insert(record.name.down(), node.get_object());
            } else {
                entries.remove(record.name.down());
            }
            save_unlocked();
            store_mutex.unlock();
        }

        /**
         * Removes custom values for an app by name.
         */
        public void remove(string name) {
            store_mutex.lock();
            var key = name.down();
            if (entries.contains(key)) {
                entries.remove(key);
                save_unlocked();
                debug("Removed custom values for %s", name);
            }
            store_mutex.unlock();
        }

        /**
         * Looks up stored custom values for an app by name.
         * Returns null if no custom values exist.
         */
        public Json.Object? lookup(string name) {
            store_mutex.lock();
            var result = entries.get(name.down());
            store_mutex.unlock();
            return result;
        }

        /**
         * Returns true if the store has any entries.
         */
        public bool has_entries() {
            store_mutex.lock();
            var result = entries.size() > 0;
            store_mutex.unlock();
            return result;
        }

        /**
         * Migrates custom values from installation records loaded from installations.json.
         * Called once during the transition from the old single-file format.
         * Only imports values that don't already exist in the store.
         */
        public void migrate_from_records(InstallationRecord[] records) {
            store_mutex.lock();
            bool changed = false;
            foreach (var record in records) {
                if (record.has_custom_values()) {
                    var key = record.name.down();
                    if (!entries.contains(key)) {
                        var node = record.to_custom_values_json();
                        entries.insert(key, node.get_object());
                        changed = true;
                        debug("Migrated custom values for %s to custom.json", record.name);
                    }
                }
            }
            if (changed) {
                save_unlocked();
            }
            store_mutex.unlock();
        }

        /**
         * Migrates custom values from history entries (uninstalled apps).
         * Only imports values that don't already exist in the store.
         */
        public void migrate_from_history(HashTable<string, Json.Object> history) {
            store_mutex.lock();
            bool changed = false;
            foreach (var key in history.get_keys()) {
                if (!entries.contains(key)) {
                    entries.insert(key, history.get(key));
                    changed = true;
                    debug("Migrated history custom values for %s to custom.json", key);
                }
            }
            if (changed) {
                save_unlocked();
            }
            store_mutex.unlock();
        }

        /**
         * Reloads custom values from disk.
         */
        public void reload() {
            store_mutex.lock();
            entries = new HashTable<string, Json.Object>(GLib.str_hash, GLib.str_equal);
            load();
            store_mutex.unlock();
        }

        /**
         * Persists current in-memory state to disk.
         */
        public void persist() {
            store_mutex.lock();
            save_unlocked();
            store_mutex.unlock();
        }

        private void load() {
            if (!store_file.query_exists(null)) {
                return;
            }
            try {
                var path = store_file.get_path();
                if (path == null) {
                    return;
                }
                string contents;
                if (!GLib.FileUtils.get_contents(path, out contents)) {
                    warning("Failed to read custom values file %s", path);
                    return;
                }
                var parser = new Json.Parser();
                parser.load_from_data(contents, contents.length);
                var root = parser.get_root();
                if (root != null && root.get_node_type() == Json.NodeType.OBJECT) {
                    var root_obj = root.get_object();
                    if (root_obj.has_member("entries")) {
                        var arr = root_obj.get_array_member("entries");
                        foreach (var node in arr.get_elements()) {
                            if (node.get_node_type() == Json.NodeType.OBJECT) {
                                var obj = node.get_object();
                                if (obj.has_member("name")) {
                                    var name = obj.get_string_member("name");
                                    entries.insert(name.down(), obj);
                                }
                            }
                        }
                    }
                }
            } catch (Error e) {
                warning("Failed to load custom values: %s", e.message);
            }
        }

        private void save_unlocked() {
            try {
                var builder = new Json.Builder();
                builder.begin_object();
                builder.set_member_name("entries");
                builder.begin_array();
                foreach (var obj in entries.get_values()) {
                    var node = new Json.Node(Json.NodeType.OBJECT);
                    node.set_object(obj);
                    builder.add_value(node);
                }
                builder.end_array();
                builder.end_object();
                var generator = new Json.Generator();
                generator.set_root(builder.get_root());
                generator.set_pretty(true);
                var json = generator.to_data(null);
                FileUtils.set_contents(store_file.get_path(), json);
            } catch (Error e) {
                warning("Failed to save custom values: %s", e.message);
            }
        }
    }
}
