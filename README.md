# JazzLight
JazzLight is a simple and clear dynamically programming language written in Rust.

## Why?
This language written for learning purposes and as target for Jazz language and maybe other language can target JazzLight VM bytecode or JazzLight.



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

