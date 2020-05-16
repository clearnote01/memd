# memd
A simple data store for key:val pairs

```
CLI to start memd datastore or fetch/store from it

USAGE:
    memd [OPTIONS] [SUBCOMMAND]

FLAGS:
        --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -h, --host <host>    hostname for the tcp server [default: 127.0.0.1]
    -p, --port <port>    port number for the tcp server [default: 7000]

SUBCOMMANDS:
    daemon    Run as the daemon
    fetch     fetch val for a key
    help      Prints this message or the help of the given subcommand(s)
    store     store key:val pair
```
