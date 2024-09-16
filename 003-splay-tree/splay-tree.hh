#pragma once

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cassert>
#include <tuple>

template<typename key_t, typename value_t>
struct splay_tree_node {
	key_t key;
	value_t value;

	struct splay_tree_node<key_t, value_t> *parent = nullptr, *lhs = nullptr, *rhs = nullptr;
};

template<typename key_t, typename value_t>
struct splay_tree {
	using node_t = struct splay_tree_node<key_t, value_t>;
private:
	node_t *root = nullptr;
	size_t size = 0;

public:
	void insert(key_t key, value_t value) {
		node_t **p = &root;
		node_t *parent = nullptr;
		while (*p != nullptr) {
			parent = *p;
			if ((*p)->key == key) {
				(*p)->value = value;
				return;
			} else if (key < (*p)->key) {
				p = &((*p)->lhs);
			} else {
				p = &((*p)->rhs);
			}
		}

		node_t *n = new node_t();
		n->key = key;
		n->value = value;
		n->parent = parent;
		n->lhs = nullptr;
		n->rhs = nullptr;
		*p = n;
	}

	value_t* lookup(key_t key, int *depth) {
		*depth = 0;
		node_t *node = root;
		while (node != nullptr && node->key != key) {
			*depth += 1;
			if (key < node->key)
				node = node->lhs;
			else
				node = node->rhs;
		}

		if (node) {
			this->splay(node);
			return &(node->value);
		}

		return nullptr;
	}

private:

	inline node_t* rotate_lhs_up(node_t *y) {
		node_t *x = y->lhs;
		if (!x)
			return nullptr;

		node_t *z = x->rhs;
		y->lhs = z;
		if (z)
			z->parent = y;

		x->parent = y->parent;
		x->rhs = y;
		y->parent = x;

		if (x->parent) {
			if (x->parent->lhs == y)
				x->parent->lhs = x;
			else
				x->parent->rhs = x;
		}

		return x;
	}

	inline node_t* rotate_rhs_up(node_t *y) {
		node_t *x = y->rhs;
		if (!x)
			return nullptr;

		node_t *z = x->lhs;
		y->rhs = z;
		if (z)
			z->parent = y;

		x->parent = y->parent;
		x->lhs = y;
		y->parent = x;

		if (x->parent) {
			if (x->parent->lhs == y)
				x->parent->lhs = x;
			else
				x->parent->rhs = x;
		}

		return x;
	}

	void splay(node_t *node) {
		while (node->parent) {
			node_t *p1 = node->parent, *p2 = node->parent->parent;
			if (!p2) {
				if (node->key < p1->key)
					this->rotate_lhs_up(p1);
				else
					this->rotate_rhs_up(p1);
				break;
			} else if (node->key < p1->key) {
				if (p1->key < p2->key) {
					this->rotate_lhs_up(p1);
					this->rotate_lhs_up(p2);
					continue;
				} else {
					this->rotate_lhs_up(p1);
					this->rotate_rhs_up(p2);
					continue;
				}
			} else if (node->key > p1->key) {
				if (p1->key > p2->key) {
					this->rotate_rhs_up(p1);
					this->rotate_rhs_up(p2);
					continue;
				} else {
					this->rotate_rhs_up(p1);
					this->rotate_lhs_up(p2);
					continue;
				}
			} else {
				assert(0);
			}
		}

		this->root = node;
	}
};



