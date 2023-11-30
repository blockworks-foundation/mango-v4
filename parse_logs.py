import sys
import re

def parse_log():
    instruction_stack = []

    string = ""

    with open(sys.argv[1], 'r') as file:
        for line in file:
            # Check if the line contains an instruction
            instruction_match = re.search(r'Instruction: (.+)', line)
            if instruction_match:
                instruction = instruction_match.group(1)
                instruction_stack.append(instruction)
            
            # Check if the line contains compute units consumed
            consumed_match = re.search(r'Program (.+) consumed (\d+) of (\d+) compute units', line)
            if consumed_match:
                program = consumed_match.group(1)
                consumed_units = int(consumed_match.group(2))
                
                # Pop the instruction from the stack and pair it with consumed units
                if instruction_stack:
                    paired_instruction = instruction_stack.pop()
                    tabs = "\t" * len(instruction_stack)
                    string = f"{tabs}Instruction: {paired_instruction}, Program: {program}, Consumed Units: {consumed_units}\n" + string
                
                if len(instruction_stack) == 0:
                    print(string)
                    string = ""

parse_log()
