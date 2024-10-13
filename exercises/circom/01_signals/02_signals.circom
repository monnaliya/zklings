pragma circom 2.1.6;

template Fun() {
    signal input in1;
    signal input in2;

    // TODO define the intermediate signal that is used to have in1 * in2
    signal intermediate;
    signal output out;

    intermediate <== in1 * in2;
    out <== intermediate + 4;
}

component main = Fun();