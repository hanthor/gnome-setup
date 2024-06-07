# setup

A description of this project.

# Distro Customization

Setup tries to customize itself based on the distro, in the following ways.

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
