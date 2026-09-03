desktop_conf = configuration_data()
desktop_conf.set('bindir', join_paths(get_option('prefix'), get_option('bindir')))

desktop_file = configure_file(
  input: 'app-manager.desktop.in',
  output: 'com.github.AppManager.desktop',
  configuration: desktop_conf
)

install_data(desktop_file,
  install_dir: join_paths(get_option('datadir'), 'applications')
)

install_data('com.github.AppManager.metainfo.xml',
  install_dir: join_paths(get_option('datadir'), 'metainfo')
)

gsettings_schemas = files('com.github.AppManager.gschema.xml')
schema_dir = join_paths(get_option('datadir'), 'glib-2.0', 'schemas')

install_data(gsettings_schemas,
  install_dir: schema_dir
)

# Pre-compile GSettings schema at build time so the package works without post-install hooks
compiled_schemas = custom_target('gschemas.compiled',
  input: gsettings_schemas,
  output: 'gschemas.compiled',
  command: [
    find_program('glib-compile-schemas'),
    '--targetdir=@OUTDIR@',
    meson.current_source_dir()
  ],
  install: true,
  install_dir: schema_dir
)

# Use gnome.post_install() for standard system integration (desktop DB, icon cache, schemas)
gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: true,
  update_desktop_database: true
)

install_data('icons/hicolor/scalable/apps/com.github.AppManager.svg',
  install_dir: join_paths(get_option('datadir'), 'icons', 'hicolor', 'scalable', 'apps')
)
