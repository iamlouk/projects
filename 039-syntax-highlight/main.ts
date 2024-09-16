export interface LanguageDesc {
  keywords?: RegExp;
  comments?: RegExp;
  constants?: RegExp;
  operators?: RegExp;
  types?: RegExp;
  strings?: RegExp;
  identifier?: RegExp;
}

export enum Kind {
  OTHER, KEYWORD, COMMENT, OPERATOR,
  CONSTANT, STRING, TYPE, IDENTIFIER,
}

export interface Part { kind: Kind; data: string }

export const languageC: LanguageDesc = {
  keywords: /^(case|const|continue|default|do|else|extern|goto|if|for|inline|register|restrict|return|static|switch|typedef|typeof|while)(?=[^A-Za-z_0-9])/,
  comments: /^(\/\/[^\n$]*|#[^\n$]*)/,
  constants: /^(true|false|(\d[\d\w_\.]*)|([A-Z_][A-Z_0-9]*))(?=[^A-Za-z_0-9])/,
  types: /^(void|signed|unsigned|char|half|int|long|struct|enum|union|bool|float|double|(\w[\w\d\_]*\_t))(?=[^A-Za-z_0-9])/,
  operators: /^[+\-\*\/%?:=<>&|\[\]\(\)\{\};]+/,
  strings: /^("(\\"|[^"])*"|'(\\'|[^'])*')/,
  identifier: /^\w[\d\w\_]*/
};

export const languageUnknown: LanguageDesc = {
  comments: /^(\/\/[^\n$]*|#[^\n$]*)/,
  constants: /^(true|false|(\d[\d\w_\.]*)|([A-Z_][A-Z_0-9]*))(?=[^A-Za-z_0-9])/,
  strings: /^("(\\"|[^"])*"|'(\\'|[^'])*'|`(\\`|[^`])*`)/,
  identifier: /^\w[\d\w\_]*/
};

export function categorize(source: string, lang: LanguageDesc): Part[][] {
  let lines: Part[][] = [];
  const checks: { kind: Kind, re: RegExp }[] = [];
  for (let key in lang) {
    let kind = ({
      "keywords": Kind.KEYWORD,
      "comments": Kind.COMMENT,
      "operators": Kind.OPERATOR,
      "constants": Kind.CONSTANT,
      "strings": Kind.STRING,
      "types": Kind.TYPE,
      "identifier": Kind.IDENTIFIER
    })[key];
    if (!kind)
      throw new Error(`Unknown language desc. key: '${key}'`);
    let re = (lang as any)[key] as RegExp;
    if (!(re instanceof RegExp))
      throw new Error(`Not a regular expression in language desc. key: '${key}'`);
    checks.push({ kind, re });
  }

  for (let line of source.split('\n')) {
    let i = 0, j = 0, parts: Part[] = [];
    while (i < line.length) {
      let matched: boolean = false;
      let restOfLine = line.substring(i);
      for (let check of checks) {
        let res = restOfLine.match(check.re);
        if (res == null)
          continue;

        if (j != i)
          parts.push({ kind: Kind.OTHER, data: line.substring(j, i) })

        let n = res[0].length;
        console.assert(n > 0, "zero-length match", check.re);
        parts.push({ kind: check.kind, data: restOfLine.substring(0, n) });
        matched = true;
        i += n;
        j = i;
        break;
      }

      if (!matched)
        i += 1;
    }

    if (j < i)
      parts.push({ kind: Kind.OTHER, data: line.substring(j, i) })

    lines.push(parts);
  }

  while (lines.length > 0 && lines[lines.length - 1].length == 0)
    lines.pop();

  return lines;
}

const escape = (unsafe: string): string =>
    unsafe
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#039;");

// TODO: Escape any HTML sequences?!
export function tohtml(lang: string, lines: Part[][]): string {
  const classes = {
    [Kind.KEYWORD]: "kw",
    [Kind.COMMENT]: "comment",
    [Kind.OPERATOR]: "op",
    [Kind.CONSTANT]: "cst",
    [Kind.STRING]: "str",
    [Kind.TYPE]: "ty",
    [Kind.IDENTIFIER]: "id"
  };

  let text: string[] = [`<div class="highlight" data-lang="${lang}"><div>`];
  for (let i = 0; i < lines.length; i += 1)
    text.push(`<b class="ln">${(i + 1)}</b>`);
  text.push(`</div>\n<div><pre>\n`);
  for (let i = 0; i < lines.length; i += 1) {
    let line = lines[i];
    for (let part of line) {
      if (part.kind == Kind.OTHER) {
        text.push(escape(part.data));
      } else {
        text.push(`<b class="`);
        text.push(classes[part.kind]);
        text.push(`">`);
        text.push(escape(part.data));
        text.push(`</b>`);
      }
    }
    text.push(`\n`);
  }
  text.push(`</pre></div></div>\n`);
  return text.join('');
}

if (import.meta.main) {
  // const tempalte: string = await Deno.readTextFile('./test-template.html');
  const tempalte: string = '{{REPLACEME}}'
  const data = await Deno.readAll(Deno.stdin);
  const code = (new TextDecoder()).decode(data);

  let highlighted = tohtml('C', categorize(code, languageC));
  let res = tempalte.replace('{{REPLACEME}}', highlighted);
  await Deno.writeAll(Deno.stdout, (new TextEncoder()).encode(res));
}
