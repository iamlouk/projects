import { assertEquals } from "https://deno.land/std@0.224.0/assert/assert_equals.ts";
import { categorize, languageC, Kind } from "./main.ts";

const SOURCES_FOO: string = `// A simple example!
float foo(size_t N, float const *A) {
  float sum = 0.f;
  for (size_t i = 0; i < N; i++)
    sum += A[i];

  // Yet another comment.
  printf("sum: %f\\n", sum);
  return sum;
}`;

Deno.test(function string1() {
  assertEquals(
    categorize("'Hello'", languageC),
    [[{ kind: Kind.STRING, data: "'Hello'" }]]);
});

Deno.test(function string2() {
  assertEquals(
    categorize('"He said: \\"Hello\\"!"', languageC),
    [[{ kind: Kind.STRING, data: '"He said: \\"Hello\\"!"' }]]);
});

Deno.test(function comment1() {
  assertEquals(
    categorize('// hello world', languageC),
    [[{ kind: Kind.COMMENT, data: '// hello world' }]]);
});

Deno.test(function foo() {
  assertEquals(
    categorize(SOURCES_FOO, languageC), [
      [
        { data: "// A simple example!", kind: Kind.COMMENT }
      ],
      [
        { data: "float", kind: Kind.TYPE },
        { data: " ", kind: Kind.OTHER },
        { data: "foo", kind: Kind.IDENTIFIER },
        { data: "(", kind: Kind.OPERATOR },
        { data: "size_t", kind: Kind.TYPE },
        { data: " ", kind: Kind.OTHER },
        { data: "N", kind: Kind.CONSTANT },
        { data: ", ", kind: Kind.OTHER },
        { data: "float", kind: Kind.TYPE },
        { data: " ", kind: Kind.OTHER },
        { data: "const", kind: Kind.TYPE },
        { data: " ", kind: Kind.OTHER },
        { data: "*", kind: Kind.OPERATOR },
        { data: "A", kind: Kind.CONSTANT },
        { data: ")", kind: Kind.OPERATOR },
        { data: " ", kind: Kind.OTHER },
        { data: "{", kind: Kind.OPERATOR },
      ],
      [
        { data: "  ", kind: Kind.OTHER },
        { data: "float", kind: Kind.TYPE },
        { data: " ", kind: Kind.OTHER },
        { data: "sum", kind: Kind.IDENTIFIER },
        { data: " ", kind: Kind.OTHER },
        { data: "=", kind: Kind.OPERATOR },
        { data: " ", kind: Kind.OTHER },
        { data: "0.f", kind: Kind.CONSTANT },
        { data: ";", kind: Kind.OPERATOR },
      ],
      [
        { data: "  ", kind: Kind.OTHER },
        { data: "for", kind: 1 },
        { data: " ", kind: Kind.OTHER },
        { data: "(", kind: Kind.OPERATOR },
        { data: "size_t", kind: Kind.TYPE },
        { data: " i ", kind: Kind.OTHER },
        { data: "=", kind: Kind.OPERATOR },
        { data: " 0", kind: Kind.OTHER },
        { data: ";", kind: Kind.OPERATOR },
        { data: " i ", kind: Kind.OTHER },
        { data: "<", kind: Kind.OPERATOR },
        { data: " ", kind: Kind.OTHER },
        { data: "N", kind: Kind.CONSTANT },
        { data: ";", kind: Kind.OPERATOR },
        { data: " i", kind: Kind.OTHER },
        { data: "++)", kind: Kind.OPERATOR },
      ],
      [
        { data: "    ", kind: Kind.OTHER },
        { data: "sum", kind: Kind.IDENTIFIER },
        { data: " ", kind: Kind.OTHER },
        { data: "+=", kind: Kind.OPERATOR },
        { data: " ", kind: Kind.OTHER },
        { data: "A", kind: Kind.CONSTANT },
        { data: "[", kind: Kind.OPERATOR },
        { data: "i", kind: Kind.OTHER },
        { data: "];", kind: Kind.OPERATOR },
      ],
      [],
      [
        { data: "  ", kind: Kind.OTHER },
        { data: "// Yet another comment.", kind: 2 },
      ],
      [
        { data: "  ", kind: Kind.OTHER },
        { data: "printf", kind: Kind.IDENTIFIER },
        { data: "(", kind: Kind.OPERATOR },
        { data: '"sum: %f\\n"', kind: 5 },
        { data: ", ", kind: Kind.OTHER },
        { data: "sum", kind: Kind.IDENTIFIER },
        { data: ");", kind: Kind.OPERATOR },
      ],
      [
        { data: "  ", kind: Kind.OTHER },
        { data: "return", kind: 1 },
        { data: " ", kind: Kind.OTHER },
        { data: "sum", kind: Kind.IDENTIFIER },
        { data: ";", kind: Kind.OPERATOR },
      ],
      [
        { data: "}", kind: Kind.OPERATOR },
      ]
  ]);
});
