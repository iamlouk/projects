import * as code from "../039-syntax-highlight/main.ts"

const isWhitespace = (c: string) => /^\s+$/.test(c);

const normalizeWhitespace = (s: string): string => {
  return s.replace(/\s\s+/g, ' ');
}

const countLeadingWhitespace = (s: string): number => s.length - s.trimStart().length

function parseLink(text: string, html: string[]): number {
  console.assert(text.charAt(0) == '[');
  const rbracket = text.indexOf('](');
  if (rbracket == -1)
    throw new Error("No closing ']' for link");

  let name: string[] = [];
  if (!parseInline(text.substring(1, rbracket), name))
    throw new Error("Invalid link text");

  const rparen = text.indexOf(')', rbracket + 2);
  if (rparen == -1)
    throw new Error("No closing ')' for link");

  const link = text.substring(rbracket + 2, rparen);
  html.push(`<a href="${link}">`);
  html.push(...name);
  html.push(`</a>`);
  return rparen + 1;
}

function parseInline(text: string, html: string[]): boolean {
  let i = 0, j = 0;
  while (i < text.length) {
    const c = text.charAt(i);
    const newword = i == 0 || isWhitespace(text.charAt(i - 1));
    if (newword && c == '[') {
      html.push(normalizeWhitespace(text.substring(j, i)));
      i += parseLink(text.substring(i), html);
      j = i;
      continue;
    }

    if (newword && c == '`') {
      html.push(normalizeWhitespace(text.substring(j, i)));
      i += 1;
      let idx = text.indexOf('`', i);
      if (idx == -1)
        throw new Error("No closing '`' for inline code block");

      html.push(`<code>`, text.substring(i, idx), `</code>`);
      i = idx + 1;
      j = i;
      continue;
    }

    if (newword && c == '*') {
      html.push(normalizeWhitespace(text.substring(j, i)));
      i += 1;
      let idx = text.indexOf('*', i);
      if (idx == -1)
        throw new Error("No closing '*' for inline highlight block");

      html.push(`<i>`);
      parseInline(text.substring(i, idx), html);
      html.push(`</i>`);
      i = idx + 1;
      j = i;
      continue;
    }

    if (newword && c == '_' && text.charAt(i + 1) == '_') {
      html.push(normalizeWhitespace(text.substring(j, i)));
      i += 2;
      let idx = text.indexOf('__', i);
      if (idx == -1)
        throw new Error("No closing '__' for inline bold block");

      html.push(`<b>`);
      parseInline(text.substring(i, idx), html);
      html.push(`</b>`);
      i = idx + 2;
      j = i;
      continue;
    }

    i += 1;
  }

  if (i != j)
    html.push(normalizeWhitespace(text.substring(j, i)));
  return true;
}

function parseHeading(line: string, html: string[]): boolean {
  let i = 0;
  while (line.charAt(i) == '#')
    i += 1;
  if (i == 0)
    return false;
  if (i > 6)
    throw new Error("Heading with too many '#' (max. 6 allowed)");

  html.push(`<h${i}>`)
  if (!parseInline(line.substring(i).trim(), html))
    throw new Error("Heading title empty or failed to parse");
  html.push(`</h${i}>\n`)
  return true;
}

function parseHBreak(line: string, html: string[]): boolean {
  if (line.trim() == '---') {
    html.push(`\n<hr/>\n`);
    return true;
  }
  return false;
}

function parseCodeBlock(i: number, lines: string[], html: string[]): number {
  if (!lines[i].startsWith('```'))
    return 0;

  let desc = code.languageUnknown;
  let language = lines[i].substring(3).trim();
  if (language == 'c' || language == 'C')
    desc = code.languageC;
  else if (language.length > 0)
    throw new Error(`Unkown/unsupported language: ${language}`);

  i += 1;
  let raw: string[] = [];
  while (i < lines.length && lines[i].trim() != '```') {
    raw.push(lines[i]);
    i += 1;
  }

  let tokens = code.categorize(raw.join('\n'), desc);
  let highlighted = code.tohtml(language, tokens);
  html.push('\n', highlighted, '\n');
  return i;
}

function parseList(i: number, lines: string[], html: string[]): number {
  if (!lines[i].startsWith('- '))
    return 0;

  const parseSubList = (prevident: number): boolean => {
    const ident = countLeadingWhitespace(lines[i]);
    const line = lines[i].trimStart();
    if (ident < prevident || !line.startsWith('- '))
      return false;
    if (ident > prevident)
      html.push('\n<ul>\n');
    html.push(`<li>`);

    parseInline(line.substring(2), html);
    i += 1;
    while (true) {
      const nident = countLeadingWhitespace(lines[i]);
      const line = lines[i].trimStart();
      if (nident != ident + 2 || line.startsWith('- '))
        break;

      html.push(` `);
      parseInline(line, html);
      i += 1;
    }

    parseSubList(ident + 1);
    html.push(`</li>\n`);
    parseSubList(ident);
    if (ident > prevident)
      html.push('</ul>\n');
    return true;
  };

  parseSubList(-1);
  return i;
}

export function toHTML(text: string): string {
  const lines = text.split('\n');
  const html: string[] = [];
  let afterNewLine = true, inParagraph = false, i = 0, j = 0;
  for (i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    if (parseHeading(line, html))
      continue;

    if (parseHBreak(line, html))
      continue;

    if ((j = parseCodeBlock(i, lines, html)) != 0) {
      i = j;
      continue;
    }

    if ((j = parseList(i, lines, html)) != 0) {
      i = j;
      continue;
    }

    const newline = line.trim().length == 0;
    if (!newline && afterNewLine && !inParagraph) {
      afterNewLine = false;
      inParagraph = true;
      html.push(`<p>`);
    } else if (newline && inParagraph) {
      afterNewLine = true;
      inParagraph = false;
      html.push(`</p>\n`);
      continue;
    } else if (!newline && !afterNewLine) {
      html.push(' ');
    }

    parseInline(line, html);
  }

  if (inParagraph)
    html.push(`</p>\n`);
  return html.join('');
}

if (import.meta.main) {
  const input = await Deno.readAll(Deno.stdin);
  const text = (new TextDecoder()).decode(input);
  const HTML = toHTML(text);
  await Deno.writeAll(Deno.stdout, (new TextEncoder()).encode(HTML));
}
