# gnome-setup

Setup performs two important functions:

1. It acts as a graphical OS installer, to deploy an OS image from installation
   media onto internal storage
2. It will also act as the initial-setup wizard (or "out of box experience", as
   other OSs call it) to configure a new installation of the OS. This mode isn't
   implemented yet

It's built for "OEM"-style installations, where at install time an image is just
deployed to disk and all setup happens on first boot, during initial-setup.

The installer portion is built on top of `systemd-repart`, and is designed for
systemd-style A/B partitioning, though technically it should be able to install
anything that can be deployed via `systemd-repart`. It is explicitly an anti-goal
to support more "traditional" package-based installations. The workflow looks
like this: set up the partition table, deploy the image, install a bootloader,
and finally reboot.

The initial-setup portion doesn't exist yet, but the plan is for it to eventually
be flexible enough to replace gnome-initial-setup across all the distros that
use it.

# STF Grant

So far, this has been developed as a prototype under the STF grant. Only the
installer portions of this project are planned under the STF grant. Eventually,
with more time and funding, we will implement the initial-setup too.

# Building

Setup is built with Meson (version 0.62 or newer), which calls `cargo build`
for the Rust code. You need the following installed first:

- a Rust toolchain (`cargo`) and Meson with Ninja
- `blueprint-compiler`, for the `.blp` UI files
- development files for GTK 4, libadwaita (1.5 or newer), gnome-desktop and
  libappstream, plus gettext

The first build also needs network access, because the `gnome-desktop` crate is
fetched from a git branch on GNOME's GitLab (see `Cargo.toml`).

```sh
meson setup builddir
meson compile -C builddir
```

The binary is written to `builddir/src/gnome-setup`.

At runtime Setup reads `pages.conf` from `/etc/gnome-setup/`, falling back to
the install libdir, and the `systemd-repart` definitions from the installed
`definitions/` directory (see `REPART_DEFINITIONS_DIR` in `src/meson.build`). An
uninstalled build will not find them, so use `meson install` into a prefix to
try the installer pages for real.

`build-aux/org.gnome.Setup.json` is a Flatpak manifest (GNOME `master` runtime
with the `rust-stable` SDK extension) for development builds.

## Prototype state

While Setup is a prototype, `src/main.rs` sets `SETUP_OVERRIDE_IN_SESSION`,
`SETUP_OVERRIDE_OS_RELEASE_TYPE`, `SETUP_OVERRIDE_OS_EXPERIMENT` and
`SETUP_DEMO` itself at startup, and `src/window.rs` always runs the
`live-initial-setup` mode from `pages.conf`. Setting those variables from
outside therefore has no effect until the debugging overrides are removed.

# Distro Customization

Setup tries to customize itself based on the distro and environment available
around it, including in the following ways.

## `os-release` metadata

Setup uses the distro's name and logo extensively, so define those there.

Setup will also use `os-release` to determine if the distro is pre-release or
experimental software, and warn the user accordingly.

## EULA

Place an AppStream component at `$DATADIR/gnome-setup/eula.xml` with an
`agreement` of `type="eula"`. Setup will pick this file up and display the
EULA Agreement page. See `build-aux/demo-eula.xml` for an example EULA file.

The exact location of the EULA file, and the file format, are subject to change
between major GNOME releases, so make sure to test that the installer continues
to present everything correctly whenever you upgrade.

Please open issues if you need:

- more flexible formatting / layout
- multiple agreements
- optional agreements
- something else
