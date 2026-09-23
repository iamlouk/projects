#!/usr/bin/env python3

lines = []
with open("/dev/stdin") as f:
    for line in f:
        line = line.strip()
        if line == "":
            continue

        lines.append([c for c in line])

row0 = lines[0]
lines = lines[1:]
beams = set([row0.index('S')])

print(beams)
print(lines)

splits = 0

for line in lines:
    newbeams = set()
    for beam in beams:
        if line[beam] == '.':
            line[beam] = '|'
            newbeams.add(beam)
            continue

        if line[beam] == '^':
            splits += 1
            if 0 <= beam - 1:
                line[beam - 1] = '|'
                newbeams.add(beam - 1)
            if beam + 1 < len(line):
                line[beam + 1] = '|'
                newbeams.add(beam + 1) 
            continue

    beams = newbeams

print(splits)
for line in lines:
    print(''.join(line))

