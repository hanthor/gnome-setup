project('gnome-setup', 'rust',
          version: '0.1.0',
    meson_version: '>= 0.62.0',
  default_options: [ 'warning_level=2', 'werror=false', ],
)

i18n = import('i18n')
gnome = import('gnome')
prefix = get_option('prefix')
bin_dir = join_paths(prefix, get_option('bindir'))
libexec_dir = join_paths(prefix, get_option('libexecdir'))

config_dir = get_option('prefix') / get_option('libdir') / meson.project_name()

systemd_dep = dependency('systemd', version: '>= 242', required: false)
systemd_userunitdir = systemd_dep.get_variable(pkgconfig: 'systemduserunitdir',
                                               pkgconfig_define: ['prefix', prefix],
                                               default_value: prefix / 'lib' / 'systemd' / 'user')

subdir('data')
subdir('src')
subdir('po')

gnome.post_install(
     glib_compile_schemas: true,
    gtk_update_icon_cache: true,
  update_desktop_database: true,
)
