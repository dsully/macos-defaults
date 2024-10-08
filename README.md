# macos-defaults

A tool for managing macOS defaults declaratively via YAML files.

## Install

### Homebrew

```shell
brew install dsully/tap/macos-defaults
```

### Source

```shell
cargo install --git https://github.com/dsully/macos-defaults
```

## Usage

### Dump a defaults domain to YAML

```shell
# To stdout:
macos-defaults dump -d com.apple.Dock

# To a file:
macos-defaults dump -d com.apple.Dock dock.yaml

# Global domain
macos-defaults dump -g
```

### Apply defaults from a YAML file

```shell
# From a single YAML file:
macos-defaults apply dock.yaml

# From a directory with YAML files & debug logging:
macos-defaults apply -vvv ~/.config/macos-defaults/
```

### Generate shell completions

```shell
macos-defaults completions [bash|fish|zsh] > ~/.config/fish/completions/macos-defaults.fish
```

See `macos-defaults --help` for more details.

## YAML Format

```yaml
---
# This will be printed to stdout.
description: Contacts

# Use the currentHost hardware UUID to find the correct plist file.
# https://apple.stackexchange.com/questions/353528/what-is-currenthost-for-in-defaults
current_host: false

# Send a SIGTERM to one or more processes if any defaults were changed.
kill: ["Contacts", "cfprefsd"]

# A nested map of plist domains to key/value pairs to set.
data:
  # Show first name
  # 1 = before last name
  # 2 = after last name
  NSGlobalDomain:
    NSPersonNameDefaultDisplayNameOrder: 1

  # Sort by
  com.apple.AddressBook:
    ABNameSortingFormat: "sortingFirstName sortingLastName"

    # vCard format
    # false = 3.0
    # true = 2.1
    ABUse21vCardFormat: false

    # Enable private me vCard
    ABPrivateVCardFieldsEnabled: false

    # Export notes in vCards
    ABIncludeNotesInVCard: true

    # Export photos in vCards
    ABIncludePhotosInVCard: true
---
# Multiple yaml docs in single file.
description: Dock

kill: ["Dock"]

data:
  # Automatically hide and show the Dock
  com.apple.dock:
    autohide: true
```

You may also use full paths to `.plist` files instead of domain names. This is the only way to set values in /Library/Preferences/.

### Overwrite syntax

By default, the YAML will be merged against existing domains.

For example, the following config will leave any other keys on `DesktopViewSettings:IconViewSettings` untouched:
```yaml
data:
  com.apple.finder:
    DesktopViewSettings:
      IconViewSettings:
        labelOnBottom: false # item info on right
        iconSize: 80.0
```

This can be overridden by adding the key `"!"` to a dict, which will delete any keys which are not specified. For example, the following config will delete all properties on the com.apple.finder domain except for DesktopViewSettings, and likewise, all properties on `IconViewSettings` except those specified.

```yaml
data:
  com.apple.finder:
    "!": {} # overwrite!
    DesktopViewSettings:
      IconViewSettings:
        "!": {} # overwrite!
        labelOnBottom: false # item info on right
        iconSize: 80.0
```

This feature has the potential to erase important settings, so exercise caution. Running `macos-defaults apply` creates a backup of each modified plist at, for example, `~/Library/Preferences/com.apple.finder.plist.prev`.

### Array merge syntax

If an array contains the element `"..."`, it will be replaced by the contents of the existing array. Arrays are treated like sets, so elements which already exist will not be added.

For example, the following config:

```yaml
data:
  org.my.test:
    aDict:
    };
        anArray: ["foo", "...", "bar"]
```

* Prepend `"foo"` to `aDict:anArray`, if it doesn't already contain `"foo"`.
* Append `"bar"` to `aDict:anArray`, if it doesn't already contain `"bar"`.

## Examples

See my [dotfiles](https://github.com/dsully/dotfiles/tree/main/.data/macos-defaults) repository.

## On YAML

![Yelling At My Laptop](docs/YAML.jpg?raw=true)

[YAML](https://yaml.org) is not a format I prefer, but out of common formats it unfortunately had the most properties I wanted.

* [JSON](https://en.wikipedia.org/wiki/JSON) doesn't have comments and is overly verbose (JSONC/JSON5 is not common)

* [XML](https://en.wikipedia.org/wiki/XML): No.

* [INI](https://en.wikipedia.org/wiki/INI_file) is too limited.

* [TOML](https://toml.io/en/) is overly verbose and is surprisingly not that easy to work with in Rust. Deeply nested maps are poorly handled.

* [KDL](https://kdl.dev) is nice, but document oriented & needs struct annotations. Derive is implemented in the 3rd party [Knuffle](https://docs.rs/knuffel/latest/knuffel/) crate.

* [RON](https://github.com/ron-rs/ron) is Rust specific, so editor support isn't there.

* [KCL](https://kcl-lang.io), [CUE](https://cuelang.org), [HCL](https://github.com/hashicorp/hcl), too high level & not appropriate for the task.

So YAML it is.

## Inspiration

This tool was heavily inspired by and uses code from [up-rs](https://github.com/gibfahn/up-rs)
