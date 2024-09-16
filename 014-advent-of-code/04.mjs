#!/usr/bin/env node

import { readFile } from 'node:fs/promises'

let input = await readFile('./04-input.txt', { encoding: 'utf8' })

let contained = ([start1, end1], [start2, end2]) => start1 <= start2 && end2 <= end1

let res = input
	.split('\n')
	.filter(line => line.length > 0)
	.map(line => line.split(',').map(range => range.split('-').map(num => Number.parseInt(num))))
	.filter(([range1, range2]) => contained(range1, range2) || contained(range2, range1))
	.reduce((sum, _) => sum + 1, 0)


console.log(res)

