#pragma once

#include <cstddef>
#include <cstdint>
#include <cassert>

#define SVL (512 / 4)

struct svbool_t {
  static constexpr size_t maxsize = SVL / 8 / 1;
  size_t size = 0;
  bool values[maxsize];

  svbool_t(size_t size, bool x): size(size) {
    for (size_t i = 0; i < maxsize; i++)
      values[i] = x;
  }

  static svbool_t whilelt(size_t esize, uint64_t a, uint64_t b) {
    svbool_t p(SVL / esize, false);
    for (uint64_t i = 0; i < p.size; i++)
      p[i] = a + i < b;
    return p;
  }

  static svbool_t whilele(size_t esize, uint64_t a, uint64_t b) {
    svbool_t p(SVL / esize, false);
    for (uint64_t i = 0; i < p.size; i++)
      p[i] = a + i <= b;
    return p;
  }

  inline bool &operator[](size_t idx) {
    assert(idx < size);
    return values[idx];
  }
};

template<typename T>
struct SVE_vector {
  using scalar_t = T;
  static constexpr size_t size = SVL / 8 / sizeof(T);
  scalar_t values[size];

  SVE_vector() {
    for (size_t i = 0; i < size; i++)
      values[i] = 0;
  }

  SVE_vector(scalar_t x) {
    for (size_t i = 0; i < size; i++)
      values[i] = x;
  }

  static SVE_vector
  load (svbool_t pg, const scalar_t *base) {
    SVE_vector<scalar_t> vec;
    assert(pg.size == vec.size);
    for (size_t i = 0; i < vec.size; i++)
      vec[i] = pg[i] ? base[i] : 0;
    return vec;
  }

  void store(svbool_t pg, scalar_t *base) {
    for (size_t i = 0; i < size; i++)
      if (pg[i])
        base[i] = values[i];
  }

  inline scalar_t &operator[](size_t idx) {
    assert(idx < size);
    return values[idx];
  }

#if 0
  SVE_vector<T> &operator += (SVE_vector<T> &rhs) {
    for (size_t i = 0; i < size; i++)
      values[i] += rhs[i];
    return *this;
  }

  SVE_vector<T> &operator -= (SVE_vector<T> &rhs) {
    for (size_t i = 0; i < size; i++)
      values[i] -= rhs[i];
    return *this;
  }

  SVE_vector<T> &operator *= (SVE_vector<T> &rhs) {
    for (size_t i = 0; i < size; i++)
      values[i] *= rhs[i];
    return *this;
  }


  SVE_vector<T> &operator /= (SVE_vector<T> &rhs) {
    for (size_t i = 0; i < size; i++)
      values[i] /= rhs[i];
    return *this;
  }
#endif
};

typedef SVE_vector<  int8_t>    svint8_t;
typedef SVE_vector< uint8_t>   svuint8_t;
typedef SVE_vector< int16_t>   svint16_t;
typedef SVE_vector<uint16_t>  svuint16_t;
typedef SVE_vector< int32_t>   svint32_t;
typedef SVE_vector<uint32_t>  svuint32_t;
typedef SVE_vector< int64_t>   svint64_t;
typedef SVE_vector<uint64_t>  svuint64_t;
typedef SVE_vector<   float> svfloat32_t;
typedef SVE_vector<  double> svfloat64_t;

static inline svbool_t svptrue_b8 () { return svbool_t(SVL /  8, true); }
static inline svbool_t svptrue_b16() { return svbool_t(SVL / 16, true); }
static inline svbool_t svptrue_b32() { return svbool_t(SVL / 32, true); }
static inline svbool_t svptrue_b64() { return svbool_t(SVL / 64, true); }

static inline uint64_t svcntb() { return SVL /  8; }
static inline uint64_t svcnth() { return SVL / 16; }
static inline uint64_t svcntw() { return SVL / 32; }
static inline uint64_t svcntd() { return SVL / 64; }

static inline svbool_t
svwhilelt_b8 (uint64_t a, uint64_t b) { return svbool_t::whilelt( 8, a, b); }
static inline svbool_t
svwhilelt_b16(uint64_t a, uint64_t b) { return svbool_t::whilelt(16, a, b); }
static inline svbool_t
svwhilelt_b32(uint64_t a, uint64_t b) { return svbool_t::whilelt(32, a, b); }
static inline svbool_t
svwhilelt_b64(uint64_t a, uint64_t b) { return svbool_t::whilelt(64, a, b); }

#define SVLD1_FOR(V) \
  static inline V \
  svld1(svbool_t pg, const V::scalar_t* base) { return V::load(pg, base); }

#define SVST1_FOR(V) \
  static inline void \
  svst1(svbool_t pg, V::scalar_t* base, V vec) { vec.store(pg, base); }

#define SVDUP_FOR(V) \
  static inline V \
  svdup(V::scalar_t x) { return V(x); }

#define SVADD_Z_FOR(V) \
  static inline V \
  svadd_z(svbool_t pg, V a, V b) {        \
    V res(0);                             \
    for (size_t i = 0; i < res.size; i++) \
      res[i] = pg[i] ? a[i] + b[i] : 0;   \
    return res;                           \
  }

#define SVMUL_Z_FOR(V) \
  static inline V \
  svmul_z(svbool_t pg, V a, V b) {        \
    V res(0);                             \
    for (size_t i = 0; i < res.size; i++) \
      res[i] = pg[i] ? a[i] * b[i] : 0;   \
    return res;                           \
  }

#define SVADD_M_FOR(V) \
  static inline V \
  svadd_m(svbool_t pg, V a, V b) {         \
    V res(0);                              \
    for (size_t i = 0; i < res.size; i++)  \
      res[i] = pg[i] ? a[i] + b[i] : a[i]; \
    return res;                            \
  }

#define SVMUL_M_FOR(V) \
  static inline V \
  svmul_m(svbool_t pg, V a, V b) {         \
    V res(0);                              \
    for (size_t i = 0; i < res.size; i++)  \
      res[i] = pg[i] ? a[i] * b[i] : a[i]; \
    return res;                            \
  }

SVLD1_FOR(   svint8_t)
SVLD1_FOR(  svuint8_t)
SVLD1_FOR(  svint16_t)
SVLD1_FOR( svuint16_t)
SVLD1_FOR(  svint32_t)
SVLD1_FOR( svuint32_t)
SVLD1_FOR(  svint64_t)
SVLD1_FOR( svuint64_t)
SVLD1_FOR(svfloat32_t)
SVLD1_FOR(svfloat64_t)

SVST1_FOR(   svint8_t)
SVST1_FOR(  svuint8_t)
SVST1_FOR(  svint16_t)
SVST1_FOR( svuint16_t)
SVST1_FOR(  svint32_t)
SVST1_FOR( svuint32_t)
SVST1_FOR(  svint64_t)
SVST1_FOR( svuint64_t)
SVST1_FOR(svfloat32_t)
SVST1_FOR(svfloat64_t)

SVDUP_FOR(   svint8_t)
SVDUP_FOR(  svuint8_t)
SVDUP_FOR(  svint16_t)
SVDUP_FOR( svuint16_t)
SVDUP_FOR(  svint32_t)
SVDUP_FOR( svuint32_t)
SVDUP_FOR(  svint64_t)
SVDUP_FOR( svuint64_t)
SVDUP_FOR(svfloat32_t)
SVDUP_FOR(svfloat64_t)

SVADD_Z_FOR(   svint8_t)
SVADD_Z_FOR(  svuint8_t)
SVADD_Z_FOR(  svint16_t)
SVADD_Z_FOR( svuint16_t)
SVADD_Z_FOR(  svint32_t)
SVADD_Z_FOR( svuint32_t)
SVADD_Z_FOR(  svint64_t)
SVADD_Z_FOR( svuint64_t)
SVADD_Z_FOR(svfloat32_t)
SVADD_Z_FOR(svfloat64_t)

SVADD_M_FOR(   svint8_t)
SVADD_M_FOR(  svuint8_t)
SVADD_M_FOR(  svint16_t)
SVADD_M_FOR( svuint16_t)
SVADD_M_FOR(  svint32_t)
SVADD_M_FOR( svuint32_t)
SVADD_M_FOR(  svint64_t)
SVADD_M_FOR( svuint64_t)
SVADD_M_FOR(svfloat32_t)
SVADD_M_FOR(svfloat64_t)

SVMUL_Z_FOR(   svint8_t)
SVMUL_Z_FOR(  svuint8_t)
SVMUL_Z_FOR(  svint16_t)
SVMUL_Z_FOR( svuint16_t)
SVMUL_Z_FOR(  svint32_t)
SVMUL_Z_FOR( svuint32_t)
SVMUL_Z_FOR(  svint64_t)
SVMUL_Z_FOR( svuint64_t)
SVMUL_Z_FOR(svfloat32_t)
SVMUL_Z_FOR(svfloat64_t)

SVMUL_M_FOR(   svint8_t)
SVMUL_M_FOR(  svuint8_t)
SVMUL_M_FOR(  svint16_t)
SVMUL_M_FOR( svuint16_t)
SVMUL_M_FOR(  svint32_t)
SVMUL_M_FOR( svuint32_t)
SVMUL_M_FOR(  svint64_t)
SVMUL_M_FOR( svuint64_t)
SVMUL_M_FOR(svfloat32_t)
SVMUL_M_FOR(svfloat64_t)


template<typename T>
struct SME_tile {
  using scalar_t = T;
  using vector_t = SVE_vector<scalar_t>;
  static constexpr size_t size = SVL / 8 / sizeof(T);

  scalar_t values[size][size];

  SME_tile(scalar_t x) {
    for (size_t i = 0; i < size; i++)
      for (size_t j = 0; j < size; j++)
        values[i][j] = x;
  }

  inline scalar_t &operator()(size_t i, size_t j) {
    assert(i < size && j < size);
    return values[i][j];
  }

  vector_t row(size_t i) {
    vector_t vec;
    assert(i < size && vec.size == size);
    for (size_t j = 0; j < size; j++)
      vec[j] = values[i][j];
    return vec;
  }

  vector_t col(size_t j) {
    vector_t vec;
    assert(j < size && vec.size == size);
    for (size_t i = 0; i < size; i++)
      vec[i] = values[i][j];
    return vec;
  }

  void row(size_t i, vector_t vec) {
    assert(i < size && vec.size == size);
    for (size_t j = 0; j < size; j++)
      values[i][j] = vec[j];
  }

  void col(size_t j, vector_t vec) {
    assert(j < size && vec.size == size);
    for (size_t i = 0; i < size; i++)
      values[i][j] = vec[i];
  }
};

struct SME_ZA {
  SME_ZA() { int8[0] = SME_tile<int8_t>(0); }

  union {
    SME_tile< int8_t>  int8[0];
    SME_tile<uint8_t> uint8[0];

    union {
      SME_tile< int16_t>  int16[2];
      SME_tile<uint16_t> uint16[2];
    };

    union {
      SME_tile< int32_t>   int32[4];
      SME_tile<uint32_t>  uint32[4];
      SME_tile<   float> float32[4];
    };

    union {
      SME_tile< int64_t>   int64[8];
      SME_tile<uint64_t>  uint64[8];
      SME_tile<  double> float64[8];
    };
  };
} SME_ZA;

static_assert(sizeof(SME_ZA) == (SVL / 8) * (SVL / 8));

void svmopa_za32_m(uint64_t tileidx,
    svbool_t pn, svbool_t pm,
    svfloat32_t zn, svfloat32_t zm) {

  assert(tileidx < sizeof(SME_ZA.float32) / sizeof(SME_ZA.float32[0]));
  SME_tile<float> &tile = SME_ZA.float32[tileidx];
  assert(pn.size == tile.size && pm.size == tile.size);
  assert(pn.size == zn.size && pm.size == zm.size);

  for (size_t i = 0; i < tile.size; i++) {
    for (size_t j = 0; j < tile.size; j++) {
      if (pn[i] && pm[j]) {
        // fprintf(stderr, "tile[%d,%d] += %f * %f\n", i, j, zn[i], zm[j]);
        tile(i, j) += zn[i] * zm[j];
      }
    }
  }
}

template<typename V>
static inline V svread_hor_za(V zd, svbool_t pg, SME_tile<typename V::scalar_t> &tile, uint32_t slice_base, uint32_t slice_offset) {
  assert(pg.size == tile.size && pg.size == zd.size);
  for (size_t i = 0; i < zd.size; i++)
    if (pg[i])
      zd[i] = tile.values[slice_base][i];

  return zd;
}

static inline svint8_t svread_hor_za8(svint8_t zd, svbool_t pg, uint64_t tileidx, uint32_t slice_base, uint32_t slice_offset) {
  assert(tileidx < sizeof(SME_ZA.int8) / sizeof(SME_ZA.int8[0]));
  SME_tile<int8_t> &tile = SME_ZA.int8[tileidx];
  return svread_hor_za<svint8_t>(zd, pg, tile, slice_base, slice_offset);
}

static inline svint32_t svread_hor_za32(svint32_t zd, svbool_t pg, uint64_t tileidx, uint32_t slice_base, uint32_t slice_offset) {
  assert(tileidx < sizeof(SME_ZA.int32) / sizeof(SME_ZA.int32[0]));
  SME_tile<int32_t> &tile = SME_ZA.int32[tileidx];
  return svread_hor_za<svint32_t>(zd, pg, tile, slice_base, slice_offset);
}

static inline svfloat32_t svread_hor_za32(svfloat32_t zd, svbool_t pg, uint64_t tileidx, uint32_t slice_base, uint32_t slice_offset) {
  assert(tileidx < sizeof(SME_ZA.float32) / sizeof(SME_ZA.float32[0]));
  SME_tile<float> &tile = SME_ZA.float32[tileidx];
  return svread_hor_za<svfloat32_t>(zd, pg, tile, slice_base, slice_offset);
}

template<typename V>
static inline void svwrite_hor_za(SME_tile<typename V::scalar_t> &tile, uint32_t slice_base, uint32_t slice_offset, svbool_t pg, V zn) {
  assert(pg.size == tile.size && pg.size == zn.size);
  for (size_t i = 0; i < zn.size; i++)
    if (pg[i])
      tile.values[slice_base][i] = zn[i];
}

static inline void svwrite_hor_za32(uint64_t tileidx, uint32_t slice_base, uint32_t slice_offset, svbool_t pg, svfloat32_t zn) {
  assert(tileidx < sizeof(SME_ZA.float32) / sizeof(SME_ZA.float32[0]));
  SME_tile<float> &tile = SME_ZA.float32[tileidx];
  svwrite_hor_za<svfloat32_t>(tile, slice_base, slice_offset, pg, zn);
}

/* TODO: #undef all these macros... */