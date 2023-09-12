all: clean test

clean:
	rm -rf obj_dir

test: struct_size_test

obj_dir:
	mkdir -p obj_dir

struct_size_test: obj_dir
	g++ tests/struct_size_test.cpp -o obj_dir/struct_size_test
	./obj_dir/struct_size_test
