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
    'this <b>is</b> <i>a</i> <a href="http://example.com">example</a> of a <code>code block</code>? <b><i>Wooh</i></b>!');
});

Deno.test(function code() {
  assertEquals(
    toHTML("hello\n```\nfoo\n```\nworld"),
    `hello\n<div class="highlight" data-lang=""><div><b class="ln">1</b></div>\n<div><pre>\n<b class="id">foo</b>\n</pre></div></div>\n\nworld`);
});
