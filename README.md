# flcheck

*flcheck* is a CLI tool to check, validate and analyze flutter (dart) package
dependencies.

[![flcheck](https://github.com/kongo2002/flcheck/actions/workflows/build.yml/badge.svg)][actions]

The tool is meant to assist when creating flutter/dart applications that are
built by assembling multiple packages into one (or multiple) applications,
sometimes called "micro frontends".

When building larger applications it is more important to control the way
packages are allowed to depend on each other. Otherwise you might end up with
one larger intertwined mess of dependencies that are hard to maintain in the
long run.

The main purpose of *flcheck* is best to be integrated into a CI/CD pipeline and
checking that certain dependency rules are met at all times. There is a
pre-configured Github action [flcheck][gh-action] in the marketplace that lets
you integrate flcheck with only a couple of lines in your github workflows.


## CI/CD integration

You can easily integrate `flcheck` into your CI/CD pipeline.

### Github workflow

For github there is even a ready-to-use action, called
[flcheck](https://github.com/marketplace/actions/flcheck). It can be as simple
as something like this:

```yaml
- name: Run flcheck
  uses: kongo2002/flcheck-action@v1
  with:
    command: validate
    config: flcheck.yaml
    appDir: ./apps
```


## Running


### Validate dependencies

```console
$ flcheck validate -d /some/dir/of/apps
```


### Print dot dependency graph

```console
$ flcheck graph -d /some/dir/of/apps > dependencies.dot
$ dot -o dependencies.png -Tpng dependencies.dot
```


### Check external dependency versions

```console
$ flcheck check -d /some/dir/of/apps
```


### Print example configuration

```console
$ flcheck example
```


## Installation

Go to the [releases page][releases], expand the list of assets and download a
ready-to-run binary.


## Building

*flcheck* is written in rust and can be built using the usual cargo toolchain:

```console
$ cargo build --release
```


## Configuration

*flcheck* expects a configuration (default `flcheck.yaml` in the current
directory) that lists the dependency rules of all packages involved.

- `package_types`: list rules for packages that describe what package is allowed
  to depend on each other
- `blacklist`: list of patterns (regular expressions) that match package
  directories that should be excluded from all validations and checks
- `validations`: configure a severity for each validation type that is
  supported (`error`, `warn` or `none`, defaults to `error`)

You *can* run flcheck without a configuration file, however not all features
will be available. If you are only interested in e.g. the dependency graph or
detect cyclic dependencies that won't be necessary though.


### Recommended package setup

The typical recommended setup is a hierachy like the following:

- `main`: the main app that is shipped and distributed to app stores and
  assembles the functionalities of one or multiple (sub) apps

- `app`: one or multiple (sub) apps that encapsulate functionalities of usually
  one domain per app - must not depend on each other

- `shared`: few shared libraries that can be used the glue together multiple
  apps, e.g. for routing, navigation - must import packages and other shared
  libraries only

- `package`: general purpose libraries (e.g. utilities) that do not contain
  domain specific logic, may be included from any package type/layer


### Example configuration

```yaml
package_types:

  main:
    # the dir_prefix is used to associate every dart package into one
    # of the package types listed here, is applied to the directory
    # name of the package
    dir_prefix: 'main'
    # list of package types all packages of this type may import from
    # (here: main is allowed to import all apps and everything that apps
    # are allowed to import themselves)
    includes:
      - app

  app:
    dir_prefix: 'app_'
    includes:
      - shared

  shared:
    dir_prefix: 'shared_'
    includes:
      - shared
      - package

  package:
    # you can also specify a list of directory prefixes
    dir_prefix:
      - 'pkg_'
      - 'util_'
    includes:
      - package

# List of patterns (regular expressions) that match package directories
# that should be excluded from all validations and checks.
# Here: exclude all auto-generated "example" packages from native dart
# packages.
blacklist:
  - '/example'

# You can configure what kind of validations are associated with what
# level of severity: `error` for errors, `warn` for warnings or
# `none` to be ignored.
# If not specified, every validation defaults to `error`
validations:
  validation:dev-dependency:git: error
  validation:dependency:unknown: error
  validation:dependency:unallowed: error
  validation:dependency:cyclic: error
  validation:public:dependency:non-git: error
```


[actions]: https://github.com/kongo2002/flcheck/actions/
[releases]: https://github.com/kongo2002/flcheck/releases/
[gh-action]: https://github.com/marketplace/actions/flcheck/
