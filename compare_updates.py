#!/usr/bin/env python3

def extract_updates_sections(filename):
    """Extract WRITE Updates and READ Updates sections from the file."""
    write_updates = ""
    read_updates = ""
    
    with open(filename, 'r') as f:
        lines = f.readlines()
    
    in_write_section = False
    in_read_section = False
    
    for i, line in enumerate(lines):
        line = line.strip()
        
        if line == "WRITE Updates":
            in_write_section = True
            in_read_section = False
            write_updates = f"WRITE Updates (line {i+1})\n"
            continue
            
        if line == "READ Updates":
            in_write_section = False
            in_read_section = True
            read_updates = f"READ Updates (line {i+1})\n"
            continue
        
        if in_write_section:
            write_updates += line + "\n"
            
        if in_read_section:
            read_updates += line + "\n"
    
    return write_updates, read_updates

def compare_sections(write_section, read_section):
    """Compare the two sections and show differences."""
    print("=" * 80)
    print("COMPARISON OF WRITE Updates vs READ Updates")
    print("=" * 80)
    
    write_lines = write_section.strip().split('\n')
    read_lines = read_section.strip().split('\n')
    
    print(f"\nWRITE Updates section ({len(write_lines)} lines):")
    print("-" * 40)
    for i, line in enumerate(write_lines[:5]):  # Show first 5 lines
        print(f"{i+1}: {line}")
    if len(write_lines) > 5:
        print(f"... ({len(write_lines) - 5} more lines)")
    
    print(f"\nREAD Updates section ({len(read_lines)} lines):")
    print("-" * 40)
    for i, line in enumerate(read_lines[:5]):  # Show first 5 lines
        print(f"{i+1}: {line}")
    if len(read_lines) > 5:
        print(f"... ({len(read_lines) - 5} more lines)")
    
    # Check if sections are identical
    if write_section == read_section:
        print("\n✅ SECTIONS ARE IDENTICAL")
    else:
        print("\n❌ SECTIONS ARE DIFFERENT")
        
        # Show differences
        print("\nDifferences:")
        print("-" * 40)
        
        # Compare line by line
        max_lines = max(len(write_lines), len(read_lines))
        for i in range(max_lines):
            write_line = write_lines[i] if i < len(write_lines) else ""
            read_line = read_lines[i] if i < len(read_lines) else ""
            
            if write_line != read_line:
                print(f"Line {i+1}:")
                print(f"  WRITE: {write_line}")
                print(f"  READ:  {read_line}")
                print()

if __name__ == "__main__":
    filename = "debug.txt"
    
    try:
        write_section, read_section = extract_updates_sections(filename)
        
        if not write_section:
            print("❌ WRITE Updates section not found")
        if not read_section:
            print("❌ READ Updates section not found")
            
        if write_section and read_section:
            compare_sections(write_section, read_section)
        else:
            print("Could not extract both sections for comparison")
            
    except FileNotFoundError:
        print(f"❌ File {filename} not found")
    except Exception as e:
        print(f"❌ Error: {e}") 