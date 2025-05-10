# Overview

This is a work-in-progress project which aims at providing a client for
interacting with an OPNsense installation. Supported APIs come from directly
from https://docs.opnsense.org/development/api.html#core-api and
https://docs.opnsense.org/development/api.html#plugins-api. If any are
missing are get added in future they can be supported by adding them to
`src/model.txt` file.

Note: this tools currently does not support POST APIs that expect a non-empty
body.

# Usage

Before you start using the client you need to have API keys. There are 2 ways
you can go about it. If you already have an API key, add them into the
`.opn.cfg` file, and add your endpoint, e.g.:

```
[default]
endpoint=https://192.168.1.1
key=<YOUR KEY>
secret=<YOUR SECRET>
```

Alternatively, you can use an artificial `login` command to create an API key
which will get stored for you in `.opn.cfg`. During the login process you'll be
asked for your username and password to authenticate API creation request. At
the end of your session you can use `logout` command to destroy your key, if
this fits your workflow.
