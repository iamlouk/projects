long fibs(long n) {
    long a = 1, b = 1;
    for (long i = 0; i < n; i = i + 1) {
        long tmp = a;
        a = a + b;
        b = tmp;
    }
    return a;
}
