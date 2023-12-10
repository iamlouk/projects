require 'ostruct'

start = OpenStruct.new(:x => 0, :y => 0, :fromstart => -1)

TOP    = "↑"
BOTTOM = "↓"
LEFT   = "←"
RIGHT  = "→"

$pipes = []
y = 0
STDIN.each_line do |line|
  $pipes.push([])
  chars = line.strip.chars
  chars.each_with_index do |c, x|
    if c == 'S'
      start = OpenStruct.new(:y => y, :x => x, :fromstart => 0)
    end
    $pipes[y].push(c)
  end
  y = y + 1
end
puts "start: #{start}"
# p pipes

def mkpos(y, x)
  return OpenStruct.new(:y => y, :x => x, :fromstart => -1)
end

def options_for(pos)
  options = []
  pipe = $pipes[pos.y][pos.x]
  case pipe
  when '|'
    options.push(mkpos(pos.y + 1, pos.x))
    options.push(mkpos(pos.y - 1, pos.x))
  when '-'
    options.push(mkpos(pos.y, pos.x + 1))
    options.push(mkpos(pos.y, pos.x - 1))
  when 'L'
    options.push(mkpos(pos.y - 1, pos.x))
    options.push(mkpos(pos.y, pos.x + 1))
  when 'J'
    options.push(mkpos(pos.y - 1, pos.x))
    options.push(mkpos(pos.y, pos.x - 1))
  when '7'
    options.push(mkpos(pos.y, pos.x - 1))
    options.push(mkpos(pos.y + 1, pos.x))
  when 'F'
    options.push(mkpos(pos.y, pos.x + 1))
    options.push(mkpos(pos.y + 1, pos.x))
  else
    return []
    # raise "WTF? How did I end up here? #{pos}"
  end
  return options
end

def walk(steps)
  pos = steps[-1]
  prev = steps[-2]
  options = options_for(pos)
  if options.size == 0
    raise "WTF? how did I end up here?" unless $pipes[pos.y][pos.x] == 'S'
    return true
  end

  options.each do |npos|
    next if npos.x == prev.x && npos.y == prev.y
    npos.fromstart = pos.fromstart + 1
    steps.push(npos)
  end
  raise "WTF? No valid option?" unless steps.last != pos
  return false
end

validstarts = []
[[-1, 0], [1, 0], [0, -1], [0, 1]].each do |offset|
  y = start.y + offset[0]
  x = start.x + offset[1]
  next if y < 0 || $pipes.size <= y
  next if x < 0 || $pipes[0].size <= x

  pos = mkpos(y, x)
  options = options_for(pos)
  # puts "options for #{pos}: #{options}"
  options.each do |opt|
    next if opt.x != start.x || opt.y != start.y
    # puts "Found valid start step: #{pos}"
    validstarts.push(pos)
  end
end

raise "WTF? Too many ways to start..." unless validstarts.size == 2

validstarts.each do |firststep|
  firststep.fromstart = 1
  steps = [start, firststep]
  while !walk(steps) do
  end
  # p steps
  puts "Steps: #{(steps.size - 1) / 2}"
end


