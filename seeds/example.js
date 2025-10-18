// Example seed for testing fuzzer
let x = 42;
let y = x + 10;

function add(a, b) {
    return a + b;
}

let result = add(x, y);
console.log(result);
