#!/usr/bin/env python3

def extract_section_content(filename, start_marker, end_marker=None):
    """Extract full content of a section from the file."""
    content = []
    in_section = False
    
    with open(filename, 'r') as file:
        for line in file:
            line = line.strip()
            
            # Check if we're entering the section
            if start_marker in line:
                in_section = True
                content.append(f"=== {line} ===")
                continue
            
            # Check if we're exiting the section (if end_marker is provided)
            if end_marker and end_marker in line and in_section:
                in_section = False
                break
            
            # If we're in the section, add the line
            if in_section:
                content.append(line)
    
    return content

def main():
    filename = "debug.txt"
    
    print("Extracting WRITE Updates section...")
    write_updates = extract_section_content(filename, "WRITE Updates", "READ Updates")
    
    print("Extracting READ Updates section...")
    read_updates = extract_section_content(filename, "READ Updates")
    
    print("\n" + "="*80)
    print("WRITE Updates Section:")
    print("="*80)
    for line in write_updates:
        print(line)
    
    print("\n" + "="*80)
    print("READ Updates Section:")
    print("="*80)
    for line in read_updates:
        print(line)
    
    print("\n" + "="*80)
    print("COMPARISON:")
    print("="*80)
    
    # Compare the content
    if write_updates == read_updates:
        print("✅ Both sections are IDENTICAL")
    else:
        print("❌ Sections are DIFFERENT")
        
        # Show differences
        print(f"\nWRITE Updates has {len(write_updates)} lines")
        print(f"READ Updates has {len(read_updates)} lines")
        
        # Find first difference
        min_len = min(len(write_updates), len(read_updates))
        for i in range(min_len):
            if write_updates[i] != read_updates[i]:
                print(f"\nFirst difference at line {i}:")
                print(f"WRITE: {write_updates[i]}")
                print(f"READ:  {read_updates[i]}")
                break

if __name__ == "__main__":
    main() 