export interface LanguageDesc {
  keywords: RegExp;
  comments: RegExp;
  constants: RegExp;
  operators: RegExp;
  types: RegExp;
  strings: RegExp;
  identifier: RegExp;
}

export enum Kind {
  OTHER, KEYWORD, COMMENT, OPERATOR,
  CONSTANT, STRING, TYPE, IDENTIFIER,
}

export interface Part { kind: Kind; data: string }

export const languageC: LanguageDesc = {
  keywords: /^(case|continue|default|do|else|extern|goto|if|for|inline|register|restrict|return|static|switch|typedef|typeof|while)/,
  comments: /^(\/\/[^\n$]*|#[^\n$]*)/,
  constants: /^(true|false|(\d[\d\w_\.]*)|([A-Z_]+))/,
  types: /^(void|signed|unsigned|char|half|int|long|struct|enum|union|bool|float|double|const|(\w[\w\d\_]*\_t))/,
  operators: /^[+\-\*\/%?:=<>&|\[\]\(\)\{\};]+/,
  strings: /^("(\\"|[^"])*"|'(\\'|[^'])*')/,
  identifier: /^\w[\d\w\_]+/
};

export function categorize(source: string, lang: LanguageDesc): Part[][] {
  let lines: Part[][] = [];
  const checks: { kind: Kind, re: RegExp }[] = [
    { kind: Kind.KEYWORD,    re: lang.keywords  },
    { kind: Kind.COMMENT,    re: lang.comments  },
    { kind: Kind.OPERATOR,   re: lang.operators  },
    { kind: Kind.CONSTANT,   re: lang.constants  },
    { kind: Kind.STRING,     re: lang.strings    },
    { kind: Kind.TYPE,       re: lang.types      },
    { kind: Kind.IDENTIFIER, re: lang.identifier },
  ];

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
export function tohtml(lines: Part[][]): string {
  const classes = {
    [Kind.KEYWORD]: "kw",
    [Kind.COMMENT]: "comment",
    [Kind.OPERATOR]: "op",
    [Kind.CONSTANT]: "cst",
    [Kind.STRING]: "str",
    [Kind.TYPE]: "ty",
    [Kind.IDENTIFIER]: "id"
  };

  let text: string[] = [`<table class="highlight">\n`];
  for (let i = 0; i < lines.length; i += 1) {
    let line = lines[i];
    text.push(`  <tr><td>${(i + 1)}</td><td><pre>`);
    for (let part of line) {
      if (part.kind == Kind.OTHER) {
        text.push(escape(part.data));
      } else {
        text.push(`<span class="`);
        text.push(classes[part.kind]);
        text.push(`">`);
        text.push(escape(part.data));
        text.push(`</span>`);
      }
    }
    text.push(`</pre></td></tr>\n`);
  }
  text.push(`</table>\n`);
  return text.join('');
}

if (import.meta.main) {
  const tempalte: string = await Deno.readTextFile('./test-template.html');
  const data = await Deno.readAll(Deno.stdin);
  const code = (new TextDecoder()).decode(data);

  let highlighted = tohtml(categorize(code, languageC));
  let res = tempalte.replace('{{REPLACEME}}', highlighted);
  await Deno.writeAll(Deno.stdout, (new TextEncoder()).encode(res));
}
