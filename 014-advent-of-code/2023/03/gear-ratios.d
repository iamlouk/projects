module adventofcode;

import std.stdio;
import std.array;
import std.algorithm;
import std.typecons;
import std.ascii;
import std.conv;
import std.range;

alias Number = Tuple!(int, "row", int, "col", int, "digits", int, "number");

@safe
bool touchesSymbol(int r, int c, bool[][] symbols) {
	return iota(-1, 2)
		.map!(dr => dr + r)
		.filter!(r => 0 <= r && r < symbols.length)
		.map!(r => iota(-1, 2)
			.map!(dc => dc + c)
			.filter!(c => 0 <= c && c < symbols[0].length)
			.map!(c => symbols[r][c])
			.any())
		.any();
}

void main(string[] args) {
	writeln("Hello, World!");

	auto input = stdin.byLineCopy().array();

	bool[][] symbols = input
		.map!(line => line.map!(c => c != '.' && (c < '0' || '9' < c)).array())
		.array();

	Number[] numbers = [];
	for (int i = 0; i < input.length; i++) {
		string line = input[i];
		for (int j = 0; j < line.length; j++) {
			int digits = 0;
			for (int k = j; k < line.length; k++) {
				if (!isDigit(line[k]))
					break;
				digits += 1;
			}

			if (digits == 0)
				continue;

			numbers ~= Number(i, j, digits, to!int(line[j..(j+digits)]));
			j += digits;
		}
	}

	int sum = numbers
		.filter!(n =>
			iota(0, n.digits).map!(i => touchesSymbol(n.row, n.col + i, symbols)).any())
		.map!(n => n.number)
		.sum();

	writeln("Sum of part numbers: ", sum);
}

