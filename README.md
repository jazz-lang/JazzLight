# JazzLight
JazzLight is a simple and clear dynamically programming language written in Rust.

## Why?
This language written for learning purposes and as target for Jazz language and maybe other language can target JazzLight VM bytecode or JazzLight.

## How to use compiler and VM
Firstly you need compile your file:
```bash
$ jazzlight file.jazz -o file.j
```
And then you can run program: 
```
$ jazzlight file.j --run
```

# Examples

factorial:
```js

function fac(x) {
    if x < 2 {
        return 1
    } else {
        return fac(x - 1) * x
    }
}

```

