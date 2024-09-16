#pragma once

#include <cstddef>
#include <cstdlib>
#include <cstdint>
#include <cstring>
#include <functional>

struct address_range {
	uint64_t first, last; /* start is inclusive, end exclusive. */
	address_range(): first(0x0), last(0x0) {}
	address_range(uint64_t first, uint64_t last): first(first), last(last) {}

	inline bool operator < (const address_range &rhs) const {
		return this->first < rhs.first && this->last <= rhs.first;
	}

	inline bool operator == (const address_range &rhs) const {
		return this->first == rhs.first && this->last == rhs.last;
	}
};

template<>
struct std::hash<struct address_range> {
	std::size_t operator()(struct address_range const &addr) const noexcept {
		return std::hash<uint64_t>{}(addr.first) ^ std::hash<uint64_t>{}(addr.last);
	}
};

template<typename T, size_t N = 2>
struct small_vec {
private:
	uint32_t size, cap;
	union {
		T *data_ptr;
		T data[N];
	};

public:
	small_vec(): size(0), cap(N) {}
	~small_vec() { if (this->cap > N) free(this->data_ptr); }

	inline void push(T val) {
		static_assert(sizeof(T) <= sizeof(void*),
				"small_vec<> is only suited for small sized elements");
		if (this->size >= this->cap) {
			T *data_old = this->cap == N ? &this->data[0] : this->data_ptr;
			T *data_new = (T*)malloc(this->cap * 4 * sizeof(T));
			memcpy(data_new, data_old, this->size * sizeof(T));
			this->cap *= 4;
		}

		if (this->cap <= N) this->data[this->size++] = val;
		else this->data_ptr[this->size++] = val;
	}

	inline T* begin() { return this->cap <= N
		? &this->data[0] : this->data_ptr; }
	inline T* end() { return this->cap <= N
		? &this->data[this->size] : this->data_ptr + this->size; }

	inline size_t get_size() const { return this->size; }
};

