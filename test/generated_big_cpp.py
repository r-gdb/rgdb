#!/bin/python3

import argparse
import sys
import random
import string
import os
import shutil
import pathlib


def main():
    parser = argparse.ArgumentParser(
        prog='ProgramName',
        description='What the program does',
        epilog='Text at the bottom of help')
    parser.add_argument('-c', '--count', type=int)      # 接受一个值的选项
    args = parser.parse_args()
    print(args)
    file_names = get_files(args.count)
    shutil.rmtree("build", ignore_errors=True)
    pathlib.Path("build/src").mkdir(parents=True)
    for file in file_names:
        # gen_file(file, "build/src")
        gen_lib(file, "build/src", file_names)
    gen_main("build/src", file_names)
    gen_cmake("build", file_names)
    # print(file_names)
    build("build")
    pass


def build(dir: str):
    pwd = os.getcwd()
    os.chdir(dir)
    os.system("du -sh src")
    os.system("cmake -version")
    os.system("mkdir build")
    os.system("cd build && cmake .. -DCMAKE_BUILD_TYPE=Debug ")
    os.system("cd build && make -j $(nproc --all)")
    os.system("ls -alh build/helloworld")
    os.system("du -sh src")
    os.system("du -sh build/helloworld")
    os.chdir(pwd)


def get_random_name(len: int) -> str:
    return ''.join((random.choice(string.ascii_letters) for x in range(len)))


def get_files(num: int) -> set[str]:
    file_names = set({})
    for i in range(num):
        name = get_random_name(10)
        while name in file_names:
            name = get_random_name(10)
        if name not in file_names:
            file_names.add(name)
        else:
            assert (False)
    return file_names


def gen_main(dir: str, funs: set[str]):
    with open("{}/main.cpp".format(dir), 'w') as f:
        f.writelines(['#include"main.hh"\n', "int main() {\n"])
        lines = ["\t{}::{}();\n".format(lib, name)
                 for lib in funs for name in funs]
        f.writelines(lines)
        f.writelines({"\treturn 0;\n"})
        f.writelines({"}\n"})
    with open("{}/main.hh".format(dir), 'w') as f:
        lines = ['#include "{}/{}.hh"\n'.format(name, name) for name in funs]
        f.writelines(lines)


def gen_lib(libname: str, dir: str, funcs: set[str]):
    work_dir = "{}/{}".format(dir, libname)
    pathlib.Path(work_dir).mkdir(parents=True)
    gen_file_lib(libname, funcs, work_dir)
    with open("{}/{}.hh".format(work_dir, libname), 'w') as f:
        f.writelines(["namespace {} {{\n".format(libname)])
        lines = ["void {}();\n".format(name) for name in funcs]
        f.writelines(lines)
        f.writelines(["}"])

    gen_cmake_lib(libname, dir)


def gen_file_lib(libname: str, funcs: set[str], dir: str):
    with open("{}/{}.cpp".format(dir, libname), 'w') as f:
        f.writelines(["namespace {} {{\n".format(libname)])
        lines = ["void {}(){{ return;}}\n".format(func) for func in funcs]
        f.writelines(lines)
        f.writelines(["}"])


def gen_cmake_lib(libname: str, dir: str):
    work_dir = "{}/{}".format(dir, libname)

    lines = '''
cmake_minimum_required(VERSION 3.5)
file(GLOB {libname}_SRC
     "*.h"
     "*.cpp"
)
add_library({libname} ${{{libname}_SRC}})
'''.format(libname=libname).splitlines()
    with open("{}/CMakeLists.txt".format(work_dir), 'w') as f:
        lines = ["{}\n".format(line) for line in lines]
        f.writelines(lines)


def gen_cmake(dir: str, sub_libs: set[str]):
    lines = '''
cmake_minimum_required(VERSION 3.5)
project(big_cpp)
file(GLOB helloworld_SRC
     "src/*.h"
     "src/*.cpp"
)
add_executable(helloworld ${helloworld_SRC})
'''.splitlines()
    with open("{}/CMakeLists.txt".format(dir), 'w') as f:
        lines = ["{}\n".format(line) for line in lines]
        f.writelines(lines)
        lines = ["add_subdirectory(src/{lib})\n".format(
            lib=lib) for lib in sub_libs]
        f.writelines(lines)
        lines = ["target_link_libraries(helloworld {})\n".format(
            lib) for lib in sub_libs]
        f.writelines(lines)


if __name__ == '__main__':
    main()
