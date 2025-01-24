# Doppler Reference

Doppler is a Domain-Specific Language (DSL) for performing operations against clusters of Lightning Network nodes across implementations.

## Basic Syntax

Doppler has a list of [`Keyword`]()s to execute [`Command`]()s that conform to a simple syntax.

A `Keyword` is a word in the program which translates to either an action to be performed on a particular node or on the cluster as a whole, or an argument to that action. We call these `Action Keyword`s and `Argument Keyword`s, respectively.

A `Variable` is a programmer-named instance of either a Node or a [`TOOL`]() in the cluster which can perform actions or have actions performed on them via `Keyword`s.

A `Command` is defined as one or more `Action Keyword`(s) and their `Action Argument`(s) which operate on one or more `Variable`(s) and can be validly interpreted as an action to be performed on a Node or `TOOL`. One `Command` is interpreted per-line.

An example of a `Command`:

> `lnd1 OPEN_CHANNEL lnd2 AMT 500000`

- `lnd1` and `lnd2` are `Variable`s
- `OPEN_CHANNEL` is an `Action Keyword`
- `AMT` is an `Argument Keyword`
- `500000` is a regular `Argument`

## Keywords 

### Cluster Keywords:

These commands perform operations relating to setting up the cluster of Nodes or `TOOL`(s) (i.e. the `docker-compose` commands):

- [`SKIP_CONF`](): If this command is used, it must be the first line in the program. Cluster configuration is already setup
It tells Doppler that the cluster configuration is already setup, and therefore any commands following it should assume that the network of nodes intended for use in the program is already setup. This function is mutually exclusive with the [UP]() command, and either `SKIP_CONF` or `UP` MUST be present in a given program. 


**Example**:
```doppler
BITCOIND_MINER bd1

LND lnd1 PAIR bd1
LND lnd2 PAIR bd1
LND lnd3 PAIR bd1

TOOL ESPLORA esp FOR bd1

UP
```

- [UP](): Spins up the cluster of nodes as defined by the `Command`s preceding it.

### Node Keywords

These `Command`s perform operations on either the `BITCOIND` or various Lightning Node implementations. The following implementations are available:

- [LND]()
- [Core Lightning]()
- [Eclair]()
