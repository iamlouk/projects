import java.io.*;

public class base64 {
  private static final int BUF_SIZE = 4096;

  private static char[] BASE64_CHARS = {
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N',
    'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b',
    'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p',
    'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3',
    '4', '5', '6', '7', '8', '9', '+', '/'
  };

  private static int encode(int n, byte[] input, char[] output, Writer outputWriter) throws IOException {
    int chunks = n / 3;
    int oidx = 0;
    for (int c = 0; c < chunks; c++) {
      int b1 = input[c * 3 + 0] & 0xff;
      int b2 = input[c * 3 + 1] & 0xff;
      int b3 = input[c * 3 + 2] & 0xff;
      output[oidx++] = BASE64_CHARS[(b1 >> 2) & 0x3f];
      output[oidx++] = BASE64_CHARS[((b1 << 4) & 0x30) | ((b2 >> 4) & 0x0f)];
      output[oidx++] = BASE64_CHARS[((b2 << 2) & 0x3c) | ((b3 >> 6) & 0x03)];
      output[oidx++] = BASE64_CHARS[(b3 & 0x3f)];
    }

    outputWriter.write(output, 0, oidx);
    return chunks * 3;
  }

  public static void encodePadding(int n, byte[] input, char[] output, Writer outputWriter) throws IOException {
    int oidx = 0;
    if (n == 1) {
      int b1 = input[0] & 0xff;
      output[oidx++] = BASE64_CHARS[(b1 >> 2) & 0x3f];
      output[oidx++] = BASE64_CHARS[(b1 << 4) & 0x30];
      output[oidx++] = '=';
      output[oidx++] = '=';
    } else if (n == 2) {
      int b1 = input[0] & 0xff;
      int b2 = input[1] & 0xff;
      output[oidx++] = BASE64_CHARS[(b1 >> 2) & 0x3f];
      output[oidx++] = BASE64_CHARS[((b1 << 4) & 0x30) | ((b2 >> 4) & 0x0f)];
      output[oidx++] = BASE64_CHARS[(b2 << 2) & 0x3c];
      output[oidx++] = '=';
    }
    outputWriter.write(output, 0, oidx);
    return;
  }

  public static void main(String[] args) throws IOException {
    var stdin = System.in;
    var stdout = new BufferedWriter(new OutputStreamWriter(System.out));

    var inputbuf = new byte[BUF_SIZE];
    var outputbuf = new char[BUF_SIZE + BUF_SIZE / 2];
    int rem = 0;
    while (true) {
      int n = stdin.read(inputbuf, rem, inputbuf.length - rem);
      if (n <= 0)
        break;

      int m = encode(n + rem, inputbuf, outputbuf, stdout);
      rem = (n + rem) % 3;
      for (int i = 0; i < rem; i++)
        inputbuf[i] = inputbuf[m+i];
    }

    if (rem != 0)
      encodePadding(rem, inputbuf, outputbuf, stdout);

    stdout.close();
  }
}
