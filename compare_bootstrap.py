#!/usr/bin/env python3

def extract_bootstrap_sections(filename):
    """Extract WRITE Bootstrap and READ Bootstrap sections from the file."""
    write_bootstrap = ""
    read_bootstrap = ""
    
    with open(filename, 'r') as f:
        lines = f.readlines()
    
    in_write_section = False
    in_read_section = False
    
    for i, line in enumerate(lines):
        line = line.strip()
        
        if line == "WRITE Bootstrap":
            in_write_section = True
            in_read_section = False
            write_bootstrap = f"WRITE Bootstrap (line {i+1})\n"
            continue
            
        if line == "READ Bootstrap":
            in_write_section = False
            in_read_section = True
            read_bootstrap = f"READ Bootstrap (line {i+1})\n"
            continue
            
        if in_write_section:
            write_bootstrap += line + "\n"
            
        if in_read_section:
            read_bootstrap += line + "\n"
    
    return write_bootstrap.strip(), read_bootstrap.strip()

def compare_sections(write_section, read_section):
    """Compare the two bootstrap sections and show differences."""
    print("=" * 80)
    print("COMPARISON OF WRITE BOOTSTRAP vs READ BOOTSTRAP")
    print("=" * 80)
    
    # Split into lines for comparison
    write_lines = write_section.split('\n')
    read_lines = read_section.split('\n')
    
    print(f"\nWRITE Bootstrap has {len(write_lines)} lines")
    print(f"READ Bootstrap has {len(read_lines)} lines")
    
    # Remove the header lines for content comparison
    write_content = '\n'.join(write_lines[1:]) if len(write_lines) > 1 else ""
    read_content = '\n'.join(read_lines[1:]) if len(read_lines) > 1 else ""
    
    print(f"\nContent comparison (excluding headers):")
    print("-" * 80)
    
    if write_content == read_content:
        print("✅ CONTENT IS IDENTICAL!")
        print("The actual bootstrap data in both sections is exactly the same.")
    else:
        print("❌ CONTENT IS DIFFERENT!")
        print("The bootstrap data differs between WRITE and READ sections.")
        
        # Show first few differences
        write_words = write_content.split()
        read_words = read_content.split()
        
        min_words = min(len(write_words), len(read_words))
        differences = 0
        
        for i in range(min_words):
            if write_words[i] != read_words[i]:
                differences += 1
                if differences <= 10:  # Show first 10 differences
                    print(f"Word {i+1}: '{write_words[i]}' vs '{read_words[i]}'")
        
        if differences > 10:
            print(f"... and {differences - 10} more differences")
    
    # Find the minimum length for line-by-line comparison
    min_len = min(len(write_lines), len(read_lines))
    
    print(f"\nLine-by-line comparison (first {min_len} lines):")
    print("-" * 80)
    
    differences_found = 0
    
    for i in range(min_len):
        write_line = write_lines[i]
        read_line = read_lines[i]
        
        if write_line != read_line:
            differences_found += 1
            print(f"Line {i+1}:")
            print(f"  WRITE: {write_line}")
            print(f"  READ:  {read_line}")
            print()
    
    if differences_found == 0:
        print("No differences found in the compared lines!")
    else:
        print(f"Found {differences_found} differences in the first {min_len} lines")
    
    # Check if one section is longer than the other
    if len(write_lines) > len(read_lines):
        print(f"\nWRITE Bootstrap has {len(write_lines) - len(read_lines)} additional lines")
        print("Additional lines in WRITE Bootstrap:")
        for i in range(len(read_lines), len(write_lines)):
            print(f"  Line {i+1}: {write_lines[i]}")
    elif len(read_lines) > len(write_lines):
        print(f"\nREAD Bootstrap has {len(read_lines) - len(write_lines)} additional lines")
        print("Additional lines in READ Bootstrap:")
        for i in range(len(write_lines), len(read_lines)):
            print(f"  Line {i+1}: {read_lines[i]}")

def show_full_sections(write_section, read_section):
    """Show the full content of both sections."""
    print("\n" + "=" * 80)
    print("FULL CONTENT COMPARISON")
    print("=" * 80)
    
    write_lines = write_section.split('\n')
    read_lines = read_section.split('\n')
    
    print(f"\nWRITE BOOTSTRAP (Full - {len(write_lines)} lines):")
    print("-" * 40)
    for i, line in enumerate(write_lines):
        print(f"{i+1:2d}: {line}")
    
    print(f"\nREAD BOOTSTRAP (Full - {len(read_lines)} lines):")
    print("-" * 40)
    for i, line in enumerate(read_lines):
        print(f"{i+1:2d}: {line}")

def main():
    filename = "debug.txt"
    
    try:
        write_section, read_section = extract_bootstrap_sections(filename)
        
        if not write_section:
            print("ERROR: WRITE Bootstrap section not found!")
            return
            
        if not read_section:
            print("ERROR: READ Bootstrap section not found!")
            return
        
        print("EXTRACTED SECTIONS:")
        print("=" * 80)
        print("WRITE BOOTSTRAP:")
        print("-" * 40)
        print(write_section[:1000] + "..." if len(write_section) > 1000 else write_section)
        print("\n" + "=" * 80)
        print("READ BOOTSTRAP:")
        print("-" * 40)
        print(read_section[:1000] + "..." if len(read_section) > 1000 else read_section)
        
        compare_sections(write_section, read_section)
        show_full_sections(write_section, read_section)
        
    except FileNotFoundError:
        print(f"ERROR: File '{filename}' not found!")
    except Exception as e:
        print(f"ERROR: {e}")

if __name__ == "__main__":
    main() 