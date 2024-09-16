// A simple example!
float foo(size_t N, float const *A) {
  float sum = 0.f;
  for (size_t i = 0; i < N; i++)
    sum += A[i];

  // Yet another comment.
  printf("sum: %f\\n", sum);
  return sum;
}
