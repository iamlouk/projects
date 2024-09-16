class Solve06 {
    public static void main(String[] args) {
        var scanner = new java.util.Scanner(System.in);

        while (scanner.hasNext()) {
            var line = scanner.nextLine().trim();
            assert line.length() > 4;

            var chars = new java.util.HashSet<Character>();
            for (int i = 0; i < 4; i++) {
                chars.add(line.charAt(i));
            }

            for (int i = 4; i < line.length(); i++) {
                if (chars.size() == 4) {
                    System.out.printf("found marker: %d\n", i-1);
                    break;
                }

                chars.remove(line.charAt(i-4));
                chars.add(line.charAt(i));
            }
        }

        scanner.close();
    }
}