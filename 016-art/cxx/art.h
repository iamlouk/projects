#pragma once
#include <cassert>
#include <cstddef>
#include <cstring>
#include <cstdint>
#include <cstdlib>
#include <optional>

#include "tagged-ptr.h"

namespace art {

enum class insert_t {
	inserted,
	replaced,
	full
};

enum class kind_t {
	ART_4,
	ART_16,
	ART_32,
	ART_256,
};

template<typename T>
struct ArtNode {
	kind_t kind;
	const uint8_t *prefix;
	ArtNode(kind_t kind): kind(kind), prefix(nullptr) {}
	virtual ~ArtNode() {}

	virtual std::optional<T*> lookup(uint8_t key, size_t *poshint = nullptr) = 0;
	virtual insert_t insert(uint8_t key, T val, size_t *poshint = nullptr) = 0;
};

template<typename T, size_t N>
struct LinSearchArtNode: ArtNode<T> {
private:
	uint8_t keys[N];
	T       vals[N];

public:
	LinSearchArtNode(kind_t kind): ArtNode<T>(kind) { memset(this->keys, 0x0, N); }

	std::optional<T*> lookup(uint8_t key, size_t *poshint = nullptr) override {
		for (size_t i = 0; i < N && this->keys[i] != '\0'; i++) {
			if (this->keys[i] == key) {
				if (poshint)
					*poshint = i;
				return &this->vals[i];
			}
		}
		return std::nullopt;
	}

	// It would probably be better to have two versions, one with
	// and one without a position hint argument.
	insert_t insert(uint8_t key, T val, size_t *poshint = nullptr) override {
		assert(key != '\0');
		if (poshint) {
			assert(this->keys[*poshint] == key);
			this->vals[*poshint] = val;
			return insert_t::replaced;
		}

		size_t i = 0;
		for (; i < N && this->keys[i] != '\0'; i++) {
			if (this->keys[i] == key) {
				if (poshint) *poshint = i;
				this->vals[i] = val;
				return insert_t::replaced;
			}
		}

		if (i == N)
			return insert_t::full;

		this->keys[i] = key;
		this->vals[i] = val;
		if (poshint) *poshint = i;
		return insert_t::inserted;
	}

	template<typename BiggerNode>
	void grow_into(BiggerNode* newnode) {
		for (size_t i = 0; i < N && this->keys[i] != '\0'; i++) {
			insert_t status = newnode->insert(this->keys[i], this->vals[i]);
			(void) status;
			assert(status == insert_t::inserted);
		}
	}
};

// T needs to be comparable to 0x0 and should be trivially to copy.
template<typename T>
struct ArtNode256: ArtNode<T> {
private:
	T       vals[256];

public:
	ArtNode256(): ArtNode<T>(kind_t::ART_256) { memset(this->vals, 0x0, 256); }

	std::optional<T*> lookup(uint8_t key, size_t *poshint = nullptr) override {
		(void) poshint;
		assert(key != '\0');
		T *val = &this->vals[key];
		return *val == 0x0 ? std::nullopt : std::optional<T*>(val);
	}

	insert_t insert(uint8_t key, T val, size_t *poshint = nullptr) override {
		(void) poshint;
		assert(key != '\0' && val != 0x0);
		bool was_filled = this->vals[key] != 0x0;
		this->vals[key] = val;
		return was_filled ? insert_t::replaced : insert_t::inserted;
	}

	template<typename BiggerNode>
	void grow_into(BiggerNode* newnode) {
		(void) newnode;
		assert(false && "impossible, a node of size 256 cannot grow");
	}
};

template<typename T>
struct Art {
private:
	using entry_t = tagged_ptr<void, T>;
	ArtNode<entry_t> *root = nullptr;

	Art(const Art<T>&) = delete;
	Art<T> &operator=(const Art<T>&) = delete;

	ArtNode<entry_t>* start_node() {
		return new LinSearchArtNode<entry_t, 4>(kind_t::ART_4);
	}

	ArtNode<entry_t>* grow(ArtNode<entry_t> *oldnode) {
		switch (oldnode->kind) {
		case kind_t::ART_4: {
			LinSearchArtNode<entry_t, 16> *newnode = new LinSearchArtNode<entry_t, 16>(kind_t::ART_16);
			static_cast<LinSearchArtNode<entry_t, 4>*>(oldnode)->grow_into(newnode);
			return newnode;
		}
		case kind_t::ART_16: {
			LinSearchArtNode<entry_t, 32> *newnode = new LinSearchArtNode<entry_t, 32>(kind_t::ART_32);
			static_cast<LinSearchArtNode<entry_t, 16>*>(oldnode)->grow_into(newnode);
			return newnode;
		}
		case kind_t::ART_32: {
			ArtNode256<entry_t> *newnode = new ArtNode256<entry_t>();
			static_cast<ArtNode256<entry_t>*>(oldnode)->grow_into(newnode);
			return newnode;
		}
		default:
			assert(false);
			return nullptr;
		}
	}

	ArtNode<entry_t>* grow_and_replace(ArtNode<entry_t> *node, uint8_t prev_key, ArtNode<entry_t> *prev_node) {
		ArtNode<entry_t>* newnode = this->grow(node);
		if (prev_node) {
			insert_t status = prev_node->insert(prev_key, newnode);
			assert(status == insert_t::replaced);
		} else {
			this->root = newnode;
		}
		return newnode;
	}

public:
	Art(): root(start_node()) {}

	std::optional<T*> lookup(const uint8_t *key) {
		size_t len = strlen((const char*)key);
		ArtNode<entry_t> *node = this->root;
		for (size_t i = 0; i < len; i++) {
			size_t poshint = 0;
			std::optional<entry_t*> res = node->lookup(key[i], &poshint);
			if (!res.has_value())
				return std::nullopt;

			entry_t entry = **res;
			if (entry.is_right())
				return entry.as_right();

			// `reinterpret_cast<decltype(...)>` is not nice, but I do not have
			// a better idea (self-referencial entry_t would be nice!).
			node = reinterpret_cast<decltype(node)>(entry.as_left());
			if (node->prefix) {
				size_t j = 0;
				for (; node->prefix[j] != '\0' && key[i] != '\0'; j++, i++)
					if (node->prefix[i] != key[i])
						return std::nullopt;
			}
		}
		return std::nullopt;
	}

	insert_t insert(const uint8_t *key, T *val) {
		size_t len = strlen((const char*)key);
		uint8_t prev_key = '\0';
		ArtNode<entry_t> *prev_node = nullptr;
		ArtNode<entry_t> *node = this->root;

		for (size_t i = 0; i < len; i++) {
			std::optional<entry_t*> next = node->lookup(key[i]);
			if (!next.has_value()) {
				ArtNode<entry_t> *newnode = nullptr;
				entry_t entry;
				if (i == len - 1) {
					entry = entry_t(val);
				} else {
					newnode = this->start_node();
					entry = entry_t(newnode);
				}

				insert_t status = node->insert(key[i], entry);
				assert(status != insert_t::replaced);
				if (status == insert_t::full) {
					node = this->grow_and_replace(node, prev_key, prev_node);
					status = node->insert(key[i], entry);
					assert(status == insert_t::inserted);
				}

				prev_key = key[i];
				prev_node = newnode;
				continue;
			}

			if ((*next)->is_right()) {
				**next = val;
				return insert_t::replaced;
			}

			assert((*next)->is_left());
			ArtNode<entry_t> *nextnode = reinterpret_cast<decltype(node)>((*next)->as_left());
			prev_node = node;
			prev_key = key[i];
			node = nextnode;
		}

		return insert_t::inserted;
	}

};

}; /* namespace art */

