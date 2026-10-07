# 实用示例

> **Status: Migration Source**
>
> 这页不再承担“主线示例集”职责。
> 当前可运行配方请改读 [18-patterns-and-recipes.md](18-patterns-and-recipes.md)。
> 本页仅保留零散旧示例和迁移线索；遇到和主线冲突的写法，应以编号章节、reference 和 spec 为准。

认证相关的最新可运行样例见 [examples/http-auth](/Users/liangzhanbo/CodeStudio/zun/examples/http-auth)。

本章节提供真实使用场景的示例，帮助你快速上手 Zun。

---

## 1. 基础计算

```zun
$>> 1 + 2 * 3;       // 7
$>> (1 + 2) * 3;     // 9
$>> 10 / 3;          // 3.3333...
$>> 10 % 3;          // 1
```

---

## 2. 字符串处理

```zun
$ name = "Zun";
$>> "Hello, " + name + "!";    // Hello, Zun!

$>> "hello world".upper();     // HELLO WORLD
$>> "  hello  ".trim();        // hello
$>> "a,b,c".split(",");        // [a, b, c]
$>> "hello".contains("ell");   // true
$>> "hello".slice(1, 3);       // el
```

**f-string 插值**

```zun
$ user = "Alice";
$ score = 100;
$>> f"Player {user} scored {score} points.";
// Player Alice scored 100 points.
```

---

## 3. 条件判断

```zun
$ age = 20;
$if age >= 18 {
    $>> "adult";
} $else {
    $>> "minor";
}
```

**多分支**

```zun
$ score = 75;
$if score >= 90 {
    $>> "A";
} $else $if score >= 80 {
    $>> "B";
} $else $if score >= 70 {
    $>> "C";
} $else {
    $>> "F";
}
```

---

## 4. 循环

**for-in 遍历**

```zun
$for item in [1, 2, 3, 4, 5] {
    $>> item;
}

$ fruits = ["apple", "banana", "cherry"];
$for fruit in fruits {
    $>> fruit;
}
```

**while 循环**

```zun
$ sum = 0;
$ i = 1;
$while i <= 100 {
    sum += i;
    i += 1;
}
$>> sum;    // 5050
```

**无限循环与 break**

```zun
$ i = 0;
$loop {
    $>> i;
    i += 1;
    $if i >= 5 {
        $break;
    }
}
```

**C 风格 for（传统写法）**

```zun
$for i = 0; i < 5; i = i + 1 {
    $>> i;
}
```

---

## 5. 函数

```zun
$fn greet(name) -> String {
    $# f"Hello, {name}!";
}

$>> greet("World");    // Hello, World!
```

**递归**

```zun
$fn factorial(n) {
    $if n <= 1 {
        $# 1;
    } $else {
        $# n * factorial(n - 1);
    }
}

$>> factorial(5);    // 120
```

**可变参数**

```zun
$fn sum(...nums) {
    $ total = 0;
    $for n in nums {
        total += n;
    }
    $# total;
}

$>> sum(1, 2, 3, 4, 5);    // 15
```

**私有函数**

```zun
_$fn helper(x) {
    $# x * 2;
}

$fn compute(n) {
    $# helper(n) + 1;
}
```

---

## 6. 列表操作

主线去向：[08-collections-and-methods.md](08-collections-and-methods.md)

```zun
$ nums = [3, 1, 4, 1, 5, 9, 2, 6];

$>> nums.len();           // 8
$>> nums.contains(5);     // true
$>> nums.index_of(4);     // 2
$>> nums.slice(2, 5);     // [4, 1, 5]
$>> nums.unique();        // [3, 1, 4, 5, 9, 2, 6]

nums.sort();
$>> nums;                 // [1, 1, 2, 3, 4, 5, 6, 9]

nums.push(10);
$>> nums.last();          // 10

$>> nums.first();         // 1
$>> nums.is_empty();      // false
```

---

## 7. Map 操作

主线去向：[08-collections-and-methods.md](08-collections-and-methods.md)

```zun
$ user = {"name": "Tom", "age": 25};

$>> user["name"];              // Tom
$>> user.keys();               // [name, age]
$>> user.contains_key("age");  // true
$>> user.len();                // 2

$for key in user {
    $>> key;
}
```

---

## 8. 异常处理

```zun
$try {
    $ result = 10 / 0;
    $>> result;
} $catch err {
    $>> f"Error: {err}";
}

主线去向：[10-error-handling.md](10-error-handling.md)
```

**主动抛出异常**

```zun
$fn divide(a, b) {
    $if b == 0 {
        $throw "division by zero";
    }
    $# a / b;
}

$try {
    $>> divide(10, 0);
} $catch err {
    $>> err;    // division by zero
}
```

---

## 9. 读写文件

```zun
$mod std.fs;

fs.write("output.txt", "Hello, Zun!");
fs.append("output.txt", "\nSecond line");

$ content = fs.read_text("output.txt");
$ lines = fs.read_lines("output.txt");

$>> content;
$for line in lines {
    $>> line;
}

fs.delete("output.txt");
```

旧的 `$>>FILE(...)` / `$<<FILE(...)` 仍可在兼容窗口中看到，但新代码应优先改用 `std.fs`。

---

## 10. 读取用户输入

```zun
$ name = $<<LINE("Enter your name: ");
$>> f"Hello, {name}!";
```

---

## 11. 使用标准库

```zun
$mod std.math;

$>> math.sqrt(16.0);        // 4.0
$>> math.floor(3.7);        // 3
$>> math.random_int(1, 6);  // 骰子：1~6
```

```zun
$mod std.fs;

fs.mkdir_all("/tmp/zun/test");
fs.write("/tmp/zun/test/hello.txt", "hi");
$ content = fs.read_text("/tmp/zun/test/hello.txt");
$>> content;    // hi
```

```zun
$mod std.json;

$ data = json.parse("{\"name\": \"Zun\", \"version\": 2026}");
$>> json.get(data, "name");      // Zun
$>> json.has(data, "version");   // true
$>> json.pretty(data);
```

---

## 12. 匿名函数

```zun
$ double = $fn(x) { $# x * 2; };
$>> double(5);    // 10
```

---

## 13. 链式调用

```zun
$>> "a,b,c".split(",").len();          // 3
$>> [3, 1, 2].sort().first();          // 1
$>> "  hello  ".trim().upper();        // HELLO
```
