# Linux

Here are the installation commands for a few Linux distributions.

## Packages

- Ubuntu 18.04 or newer / Debian stretch or newer

## udev rules

This rule lets you use OpenOCD with the Discovery board without root privilege.

Create the file `/etc/udev/rules.d/70-st-link.rules` with the contents shown below.

``` text
{{1include}}
```

Then reload all the udev rules with:

``` console
sudo udevadm control --reload-rules
```

If you had the board plugged to your laptop, unplug it and then plug it again.

You can check the permissions by running this command:

``` console
lsusb
```

Which should show something like

```text
(..)
Bus 001 Device 018: ID 0483:374b STMicroelectronics ST-LINK/V2.1
(..)
```

Take note of the bus and device numbers. Use those numbers to create a path like
`/dev/bus/usb/<bus>/<device>`. Then use this path like so:

``` console
ls -l /dev/bus/usb/001/018
```

```text
crw-------+ 1 root root 189, 17 Sep 13 12:34 /dev/bus/usb/001/018
```

```console
getfacl /dev/bus/usb/001/018 | grep user
```

```text
user::rw-
user:you:rw-
```

The `+` appended to permissions indicates the existence of an extended
permission. The `getfacl` command tells the user `you` can make use of
this device.

Now, go to the [next section].

[next section]: verify.md

## Attribution

Some pages from this book are based upon the rust-embedded book found in [this repository] which is developed by the [resources team].
This page is loosely based on this [original page].

[this repository]: https://github.com/rust-embedded/book
[resources team]: https://github.com/rust-embedded/wg#the-resources-team
[original page]: https://docs.rust-embedded.org/book/intro/install/linux.html
