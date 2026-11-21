# gnome-setup

> **This is a mirror.** The upstream project is
> <https://gitlab.gnome.org/Teams/STF/setup> (GNOME STF); `main` and the other
> upstream branches here are unmodified copies of it. Issues and merge requests
> belong upstream. The `utah` branch carries the small patches that
> [Project Bluefin's Utah](https://github.com/projectbluefin/utah) image-based
> installer builds from (pinned by commit in `iso/raw/Containerfile`), each
> meant to go upstream:
>
> - main: Don't force demo mode and the debug overrides
> - crypt-enroll: Return errors instead of panicking
> - installer: Report failures, support a distro post-install unit
>   (`[page:installer] post-install-unit=` in `pages.conf`), enroll the
>   recovery key on the installed disk

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
