import { assertEquals } from "@std/assert";
import { toHTML } from "./main.ts";

Deno.test(function empty() {
  assertEquals(toHTML(""), "");
});

Deno.test(function h2() {
  assertEquals(toHTML("## Hello!"), "<h2>Hello!</h2>\n");
});

Deno.test(function hr() {
  assertEquals(toHTML("---"), "\n<hr/>\n");
});

Deno.test(function inline() {
  assertEquals(
    toHTML("this __is__ *a* [example](http://example.com) of a `code block`? __*Wooh*__!"),
    '<p>this <b>is</b> <i>a</i> <a href="http://example.com">example</a> of a <code>code block</code>? <b><i>Wooh</i></b>!</p>\n');
});

Deno.test(function code() {
  assertEquals(
    toHTML("hello\n\n```\nfoo\n```\n\nworld"),
    `<p>hello</p>\n\n<div class="highlight" data-lang=""><div><b class="ln">1</b></div>\n<div><pre>\n<b class="id">foo</b>\n</pre></div></div>\n\n<p>world</p>\n`);
});

Deno.test(function list() {
  assertEquals(
    toHTML(`
# A list...

- foo
- bar
  - one
  - two
    - three
    - four
  - five
- hello
  world
  - six
`),
    `<h1>A list...</h1>\n\n<ul>\n<li>foo</li>\n<li>bar\n<ul>\n<li>one</li>\n<li>two\n<ul>\n<li>three</li>\n<li>four</li>\n</ul>\n</li>\n<li>five</li>\n</ul>\n</li>\n<li>hello world\n<ul>\n<li>six</li>\n</ul>\n</li>\n</ul>\n`);
});
