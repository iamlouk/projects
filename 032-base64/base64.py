# The problem is that because this module is called base64,
# when doing a plain `import base64`, this imports itself.
import sys
sys.path = sys.path[1:]

# Perf. in "raw" python would be horrible, so just use its
# stdlib instead.
import base64

input: bytes = sys.stdin.buffer.read()
output: bytes = base64.b64encode(input)
sys.stdout.buffer.write(output)
