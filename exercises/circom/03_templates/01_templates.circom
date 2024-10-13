pragma circom 2.1.6;

template A(x) {
    signal input in;
    signal output out;

    out <== x * in;
}

template Main() {
    signal input a;

    signal output out;

    // TODO: Instantiate the template A

    component instA = A(5);

    instA.in <== a;
    out <== instA.out;
}

component main = Main();