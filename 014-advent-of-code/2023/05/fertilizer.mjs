import { stdin, stdout } from 'node:process';
import { readFileSync } from 'node:fs';

const input = readFileSync(stdin.fd, 'utf8').split('\n');

const seeds = input[0].substring(7).split(' ').map(seed => Number.parseInt(seed.trim()))

const parseMaps = input => {
	const mapping = input[0];
	console.assert(mapping.endsWith(' map:'));
	const [mfrom, mto] = mapping.substring(0, mapping.length - ' map:'.length).split('-to-');
	// console.log("parsing: %s -> %s", mfrom, mto);
	const end = input.indexOf('');

	const map = input
		.slice(1, end)
		.map(line => line.split(' ').map(x => Number.parseInt(x)))
		.map(([dst, src, len]) => ({ dst, src, len }))
		.sort((a, b) => a.src - b.src)

	for (let i = 0; i < map.length - 1; i++) {
		const a = map[i], b = map[i+1];
		console.assert(a.src + a.len <= b.src);
	}

	const rest = (end + 1 < input.length)
		? parseMaps(input.slice(end + 1))
		: [];

	return [ map, ...rest ];
};

const walkMaps = (maps, seed) => maps.reduce((x, map) => {
		for (let range of map)
			if (range.src <= x && x < range.src + range.len)
				return x - range.src + range.dst;
		return x;
	}, seed);

const maps = parseMaps(input.slice(2));
for (let seed of seeds)
	console.log(seed, "->", walkMaps(maps, seed));

const min = seeds
	.map(seed => walkMaps(maps, seed))
	.reduce((a, b) => Math.min(a, b));

console.log(min);

