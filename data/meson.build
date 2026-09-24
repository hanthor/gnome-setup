desktop_file = i18n.merge_file(
        input: 'org.gnome.Setup.desktop.in',
       output: 'org.gnome.Setup.desktop',
         type: 'desktop',
       po_dir: '../po',
      install: true,
  install_dir: get_option('datadir') / 'applications'
)

desktop_utils = find_program('desktop-file-validate', required: false)
if desktop_utils.found()
  test('Validate desktop file', desktop_utils, args: [desktop_file])
endif

appstream_file = i18n.merge_file(
        input: 'org.gnome.Setup.metainfo.xml.in',
       output: 'org.gnome.Setup.metainfo.xml',
       po_dir: '../po',
      install: true,
  install_dir: get_option('datadir') / 'metainfo'
)

appstreamcli = find_program('appstreamcli', required: false, disabler: true)
test('Validate appstream file', appstreamcli,
     args: ['validate', '--no-net', '--explain', appstream_file])

install_data('org.gnome.Setup.gschema.xml',
  install_dir: get_option('datadir') / 'glib-2.0' / 'schemas'
)

compile_schemas = find_program('glib-compile-schemas', required: false, disabler: true)
test('Validate schema file',
     compile_schemas,
     args: ['--strict', '--dry-run', meson.current_source_dir()])

install_subdir(
  'definitions',
  strip_directory: true,
  install_dir:  config_dir / 'definitions',
)

install_data('pages.conf',
  install_dir: config_dir,
)

install_data(
  '20-gnome-setup.rules',
  install_dir: get_option('datadir') / 'polkit-1' / 'rules.d',
)

configure_file(
    input: 'gnome-setup.service.in',
    output: 'gnome-setup.service',
    configuration: {
        'bindir': bin_dir,
        'libexecdir': libexec_dir,
    },
    install_dir: systemd_userunitdir
)

configure_file(
    input: 'gnome-setup.session.conf.in',
    output: 'gnome-setup.conf',
    configuration: {},
    install_dir: systemd_userunitdir / 'gnome-session@gnome-initial-setup.target.d',
)

subdir('icons')
