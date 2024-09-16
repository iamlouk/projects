#include <vector>
#include <map>
#include <cstdio>
#include <string>
#include <iostream>
#include <memory>
#include <variant>

struct Entry {
	Entry(std::string name): name(name) {}

	std::string name;
	virtual size_t getSize() = 0;
	virtual ~Entry() {}
};

struct File: public Entry {
	size_t size;
	File(std::string name, size_t size): Entry(name), size(size) {}

	size_t getSize() override {
		return this->size;
	}
};

struct Directory: public Entry {
	std::vector<std::shared_ptr<Entry>> entries;
	Directory(std::string name): Entry(name), entries({}) {}

	size_t getSize() override {
		size_t sum = 0;
		for (auto &entry: entries)
			sum += entry->getSize();
		return sum;
	}
};

int main() {
	std::string line;
	std::getline(std::cin, line); // skip first line

	std::vector<std::shared_ptr<Directory>> cwd;
	cwd.push_back(std::make_shared<Directory>("/"));

	while (!std::cin.eof() && !std::cin.fail()) {
		std::getline(std::cin, line);
		if (line.length() <= 1) {
			continue;
		} else if (line == "cd ..") {
			cwd.pop_back();
		} else if (line.rfind("$ ls", 0) == 0) {
			std::shared_ptr<Directory> tos = cwd.back();

		}






	}
}

