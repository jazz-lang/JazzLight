# JazzLight
JazzLight is a simple and clear dynamically programming language written in Rust.

## Why?
This language written for learning purposes and as target for Jazz language and maybe other language can target JazzLight VM bytecode or JazzLight.

## How to use compiler and VM
Firstly you need compile your file:
```bash
$ jazzlight file.jazz
```
Then you can execute bytecode:
```bash
$ jazzlight-vm file.j
```
If you want to dump bytecode you can use compiler of `decoder` program:
```bash
$ jazzlight -d file.jazz
# Or
$ jazzlight-vm ~/.jazz/decoder file.j
```

# Example 

factorial:
```coffeescript

var fac = function(x) -> if x == 0 {
    return 1
} else {
    return fac(x - 1) * x
}


$print(fac(5))

```

Creating object:
```coffeescript

var object = $new(null)

object.x = 2

var object2  = $new(object)
$print(object.x == object2.x)

```

