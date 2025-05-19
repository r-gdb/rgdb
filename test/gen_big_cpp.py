#!/usr/bin/env python3
"""
生成一个包含十万行的C++文件，用于测试程序性能和稳定性
"""

import random
import string
import sys

OUTPUT_FILE = "generated_big_cpp.cpp"
TOTAL_LINES = 100_0000  # 十万行


def random_string(length=10):
    """生成随机字符串"""
    return ''.join(random.choices(string.ascii_letters, k=length))


def random_number(min_val=0, max_val=1000):
    """生成随机数字"""
    return random.randint(min_val, max_val)


def generate_variable_definition():
    """生成变量定义"""
    types = ["int", "float", "double", "char", "bool", "long", "unsigned int"]
    var_type = random.choice(types)
    var_name = random_string(random.randint(5, 15))
    
    if var_type == "char":
        value = f"'{random.choice(string.ascii_letters)}'"
    elif var_type == "bool":
        value = random.choice(["true", "false"])
    elif var_type == "float" or var_type == "double":
        value = f"{random.uniform(0, 1000):.6f}"
    else:
        value = str(random_number())
    
    return f"{var_type} {var_name} = {value};"


def generate_function(func_id):
    """生成函数定义"""
    return_types = ["void", "int", "float", "double", "bool", "std::string"]
    return_type = random.choice(return_types)
    func_name = f"function_{func_id}"
    
    lines = [f"{return_type} {func_name}() {{"]
    num_statements = random.randint(3, 20)
    
    for _ in range(num_statements):
        lines.append(f"    {generate_variable_definition()}")
    
    if return_type != "void":
        if return_type == "bool":
            lines.append("    return true;")
        elif return_type == "std::string":
            lines.append('    return "hello";')
        elif return_type == "float" or return_type == "double":
            lines.append("    return 3.14;")
        else:
            lines.append("    return 42;")
    
    lines.append("}")
    lines.append("")  # 空行分隔函数
    return lines


def generate_class(class_id):
    """生成类定义"""
    class_name = f"Class_{class_id}"
    
    lines = [f"class {class_name} {{"]
    lines.append("public:")
    
    # 构造函数
    lines.append(f"    {class_name}() {{")
    lines.append("        // 初始化代码")
    lines.append("    }")
    lines.append("")
    
    # 析构函数
    lines.append(f"    ~{class_name}() {{")
    lines.append("        // 清理代码")
    lines.append("    }")
    lines.append("")
    
    # 添加方法
    num_methods = random.randint(1, 5)
    for i in range(num_methods):
        method_lines = generate_function(f"{class_name}_{i}")
        # 缩进所有行
        lines.extend("    " + line for line in method_lines)
    
    # 添加成员变量
    lines.append("private:")
    num_vars = random.randint(2, 8)
    for _ in range(num_vars):
        types = ["int", "float", "double", "char", "bool", "std::string"]
        var_type = random.choice(types)
        var_name = f"m_{random_string(random.randint(5, 10))}"
        lines.append(f"    {var_type} {var_name};")
    
    lines.append("};")
    lines.append("")  # 空行分隔类
    return lines


def generate_cpp_file():
    """生成完整的C++文件"""
    lines = [
        "#include <iostream>",
        "#include <vector>",
        "#include <string>",
        "#include <algorithm>",
        "#include <cmath>",
        "#include <memory>",
        "",
        "// 这是一个自动生成的C++文件，包含十万行代码",
        "// 用于测试调试器对大文件的处理能力",
        "",
        "namespace test_namespace {",
        ""
    ]
    
    # 生成全局变量
    lines.append("// 全局变量")
    for i in range(50):
        lines.append(generate_variable_definition())
    lines.append("")
    
    # 生成一些函数前向声明
    lines.append("// 函数原型")
    for i in range(1, 101):
        lines.append(f"void forward_declaration_func_{i}();")
    lines.append("")
    
    # 生成类和函数来填充剩余的行数
    remaining_lines = TOTAL_LINES - len(lines) - 10  # 留出一些空间给main和命名空间结束
    
    class_count = 0
    function_count = 0
    
    while len(lines) < remaining_lines:
        if random.random() < 0.3:  # 30%的概率生成类
            class_lines = generate_class(class_count)
            lines.extend(class_lines)
            class_count += 1
        else:  # 70%的概率生成函数
            func_lines = generate_function(function_count)
            lines.extend(func_lines)
            function_count += 1
    
    # 添加main函数
    lines.extend([
        "} // namespace test_namespace",
        "",
        "int main() {",
        "    std::cout << \"Testing large C++ file processing...\" << std::endl;",
        "    return 0;",
        "}",
        ""
    ])
    
    return lines


def write_to_file(lines, filename):
    """将生成的内容写入文件"""
    with open(filename, 'w') as f:
        for line in lines:
            f.write(line + '\n')


def main():
    print(f"开始生成包含 {TOTAL_LINES} 行的C++文件...")
    cpp_lines = generate_cpp_file()
    
    output_file = OUTPUT_FILE
    if len(sys.argv) > 1:
        output_file = sys.argv[1]
    
    write_to_file(cpp_lines, output_file)
    print(f"生成完成！文件已保存为: {output_file}")
    print(f"实际行数: {len(cpp_lines)}")


if __name__ == "__main__":
    main()