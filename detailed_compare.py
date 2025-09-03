#!/usr/bin/env python3

def analyze_bootstrap_sections(filename):
    """Detailed analysis of WRITE Bootstrap and READ Bootstrap sections."""
    
    with open(filename, 'r') as f:
        lines = f.readlines()
    
    write_section = []
    read_section = []
    
    in_write_section = False
    in_read_section = False
    
    for i, line in enumerate(lines):
        line = line.strip()
        
        if line == "WRITE Bootstrap":
            in_write_section = True
            in_read_section = False
            write_section.append(f"Line {i+1}: {line}")
            continue
            
        if line == "READ Bootstrap":
            in_write_section = False
            in_read_section = True
            read_section.append(f"Line {i+1}: {line}")
            continue
        
        if in_write_section:
            write_section.append(f"Line {i+1}: {line}")
            
        if in_read_section:
            read_section.append(f"Line {i+1}: {line}")
    
    print("=== DETAILED BOOTSTRAP COMPARISON ===\n")
    
    print("WRITE Bootstrap Section:")
    print("-" * 50)
    for line in write_section:
        print(line)
    
    print("\nREAD Bootstrap Section:")
    print("-" * 50)
    for line in read_section:
        print(line)
    
    print("\n=== COMPARISON ANALYSIS ===")
    print(f"WRITE Bootstrap lines: {len(write_section)}")
    print(f"READ Bootstrap lines: {len(read_section)}")
    
    # Compare content (excluding headers)
    write_content = write_section[1:] if len(write_section) > 1 else []
    read_content = read_section[1:] if len(read_section) > 1 else []
    
    if write_content == read_content:
        print("✅ Content is IDENTICAL (excluding headers)")
        print(f"Data: {write_content[0] if write_content else 'No data'}")
    else:
        print("❌ Content is DIFFERENT")
        print("Write content:", write_content)
        print("Read content:", read_content)
    
    # Byte analysis
    if write_content and read_content:
        write_data = write_content[0]
        read_data = read_content[0]
        
        print(f"\n=== BYTE ANALYSIS ===")
        print(f"Write data: {write_data}")
        print(f"Read data:  {read_data}")
        
        if write_data.startswith("0x"):
            write_bytes = bytes.fromhex(write_data[2:])
            read_bytes = bytes.fromhex(read_data[2:])
            
            print(f"Write bytes length: {len(write_bytes)}")
            print(f"Read bytes length:  {len(read_bytes)}")
            print(f"All bytes zero: {all(b == 0 for b in write_bytes)}")

if __name__ == "__main__":
    analyze_bootstrap_sections("debug.txt") 