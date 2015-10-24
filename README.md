# Hokaido

Hokaido is a character based terminal sharing tool. It's speedy and clear.

## How to use

### 1. Start server

Server is needed to relay communication between broadcaster and watchers.

``` sh
$ hokaido server # on 203.0.113.10:4423
```

### 2. Watch

`watch` displays broadcaster's shell output.

``` sh
$ hokaido watch --host 203.0.113.10
```

### 3. Broadcast

`broadcast` invokes a new $SHELL and starts broadcasting its output.

``` sh
$ hokaido broadcast --host 203.0.113.10
```
