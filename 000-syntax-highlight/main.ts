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
  constants: /^(true|false|(\d[\d\w_\.]+)|([A-Z_]+))/,
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

if (import.meta.main) {
  console.log("Hello World!");
}
