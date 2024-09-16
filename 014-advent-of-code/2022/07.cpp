#include <vector>
#include <map>
#include <cstdio>
#include <string>
#include <cstring>
#include <cassert>
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
		for (auto &entry: this->entries)
			sum += entry->getSize();
		return sum;
	}

	std::shared_ptr<Entry> getEntry(std::string &name) {
		for (std::shared_ptr<Entry> &entry: this->entries)
			if (entry->name == name)
				return entry;
		return nullptr;
	}

	void addEntry(std::shared_ptr<Entry> entry) {
		this->entries.push_back(entry);
	}

	size_t getSolution(size_t max) {
		size_t size = this->getSize();
		if (size < 
	}
};

int main() {
	std::vector<std::shared_ptr<Directory>> cwd;
	std::shared_ptr<Directory> root =
		std::make_shared<Directory>("/");
	cwd.push_back(root);

	std::string line;
	std::getline(std::cin, line); // skip first line
	std::getline(std::cin, line); // get first line
	while (!std::cin.eof() && !std::cin.fail() && line.length() > 0) {
		// std::cout << "line: '" << line << "'\n";
		std::shared_ptr<Directory> tos = cwd.back();
		if (line.starts_with("$ cd ")) {
			std::string dirname = line.substr(strlen("$ cd "), line.length() - 1);
			if (dirname == "..") {
				cwd.pop_back();
			} else {
				std::shared_ptr<Directory> entry =
					std::dynamic_pointer_cast<Directory>(tos->getEntry(dirname));
				assert(entry != nullptr);
				cwd.push_back(entry);
			}
			std::getline(std::cin, line); // get next line
		} else if (line.starts_with("$ ls")) {
			for (;;) {
				std::getline(std::cin, line); // get next line
				if (line.starts_with("$") || line.length() <= 1 || std::cin.eof())
					break;
				if (line.starts_with("dir ")) {
					std::string dirname = line.substr(strlen("dir "), line.length() - 1);
					std::shared_ptr<Entry> entry = std::make_shared<Directory>(dirname);
					tos->addEntry(entry);
				} else {
					size_t pos = 0;
					int size = std::stoi(line, &pos, 10);
					std::string filename = line.substr(pos + 1, line.length() - 1);
					std::shared_ptr<Entry> entry = std::make_shared<File>(filename, size);
					tos->addEntry(entry);
				}
			}
		} else {
			assert(0);
		}
	}
}

