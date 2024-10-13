pragma circom 2.1.6;

template Poly() {
    signal input x;
    signal input y;
    signal input z;

    signal intermediate1;
    signal intermediate2;
    signal intermediate3;
    signal output out;

    intermediate1 <== x * y;
    intermediate2 <== intermediate1 * z;
    intermediate3 <== intermediate2 + 4;
    out <== intermediate3;
}

component main = Poly();