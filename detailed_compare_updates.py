#!/usr/bin/env python3

def analyze_updates_sections(filename):
    """Detailed analysis of WRITE Updates and READ Updates sections."""
    
    with open(filename, 'r') as f:
        lines = f.readlines()
    
    write_section = []
    read_section = []
    
    in_write_section = False
    in_read_section = False
    
    for i, line in enumerate(lines):
        line = line.strip()
        
        if line == "WRITE Updates":
            in_write_section = True
            in_read_section = False
            write_section.append(f"Line {i+1}: {line}")
            continue
            
        if line == "READ Updates":
            in_write_section = False
            in_read_section = True
            read_section.append(f"Line {i+1}: {line}")
            continue
            
        if in_write_section:
            write_section.append(f"Line {i+1}: {line}")
            
        if in_read_section:
            read_section.append(f"Line {i+1}: {line}")
    
    print("=== WRITE Updates Section ===")
    for line in write_section:
        print(line)
    
    print("\n=== READ Updates Section ===")
    for line in read_section:
        print(line)
    
    print("\n=== Comparison ===")
    
    # Compare content (excluding headers)
    write_content = write_section[1:] if len(write_section) > 1 else []
    read_content = read_section[1:] if len(read_section) > 1 else []
    
    if write_content == read_content:
        print("✅ Content is IDENTICAL")
        print(f"Both sections contain {len(write_content)} lines of data")
        if write_content:
            print(f"Data: {write_content[0]}")
    else:
        print("❌ Content is DIFFERENT")
        print("Write content:")
        for line in write_content:
            print(f"  {line}")
        print("Read content:")
        for line in read_content:
            print(f"  {line}")

if __name__ == "__main__":
    analyze_updates_sections("debug.txt") 