#pragma once
#include <cstddef>
#include <cstdint>
#include <cassert>
namespace art {

template<typename Left, typename Right>
struct tagged_ptr {
private:
	uintptr_t val;
	static constexpr uintptr_t TAG_MASK = 1ul << ((8 * sizeof(void*)) - 1);
	static constexpr uintptr_t PTR_MASK = ~TAG_MASK;
	static_assert(PTR_MASK == 0x7fff'ffff'ffff'ffff, "mask check");
	static_assert(TAG_MASK == 0x8000'0000'0000'0000, "mask check");
public:

	tagged_ptr(): val(0x0) {}
	tagged_ptr(std::nullptr_t _): val(0x0) { (void)_;  }
	tagged_ptr(Left *ptr): val(uintptr_t(ptr)) { assert(!(uintptr_t(ptr) & TAG_MASK)); }
	tagged_ptr(Right *ptr): val(ptr == nullptr ? 0x0 : (uintptr_t(ptr) | TAG_MASK)) { assert(!(uintptr_t(ptr) & TAG_MASK)); }

	inline bool is_null()  const { return this->val == 0; }
	inline bool is_left()  const { return !is_null() && (this->val & TAG_MASK) == 0; }
	inline bool is_right() const { return !is_null() && (this->val & TAG_MASK) != 0; }

	tagged_ptr<Left, Right> operator=(Left *ptr) {
		this->val = uintptr_t(ptr);
		assert(!(uintptr_t(ptr) & TAG_MASK));
		return *this;
	}

	tagged_ptr<Left, Right> operator=(Right *ptr) {
		this->val = ptr == nullptr ? 0x0 : (uintptr_t(ptr) | TAG_MASK);
		assert(!(uintptr_t(ptr) & TAG_MASK));
		return *this;
	}

	tagged_ptr<Left, Right> operator=(std::nullptr_t _) {
		(void) _;
		this->val = 0x0;
		return *this;
	}

	inline operator Left*() { return as_left(); }
	inline operator Right*() { return as_right(); }

	inline Left* as_left() { assert(is_left()); return reinterpret_cast<Left*>(this->val); }
	inline Right* as_right() { assert(is_right()); return reinterpret_cast<Right*>(this->val); }
};

template<typename Left, typename Right>
bool operator==(const tagged_ptr<Left, Right> &lhs, uintptr_t rhs) {
	assert(rhs == 0x0);
	return lhs.is_null();
}

template<typename Left, typename Right>
bool operator!=(const tagged_ptr<Left, Right> &lhs, uintptr_t rhs) {
	return !(lhs == rhs);
}

static_assert(sizeof(struct tagged_ptr<float, double>) == sizeof(void*), "tagged_ptr size");

}; /* namespace art */

