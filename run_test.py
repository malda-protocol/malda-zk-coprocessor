import json
import re
import subprocess

def update_integration_tests(config):
    with open('methods/tests/integration_tests.rs', 'r') as f:
        content = f.read()

    # Update num_iterations
    num_iterations = config['execution_params']['num_iterations']
    content = re.sub(r'(let num_iterations = )\d+;', rf'\g<1>{num_iterations};', content)

    # Update time_delay
    time_delay = config['execution_params']['time_delay_secs']
    content = re.sub(r'(sleep\(Duration::from_secs\()\d+(\)\))', rf'\g<1>{time_delay}\2', content)

    # Build chain-related parameters
    users_vec = []
    assets_vec = []
    dst_chain_ids_vec = []
    chain_ids_vec = []
    total_users = 0
    total_users_comment_parts = []

    for chain in config['execution_params']['chains']:
        num_users = chain['num_users']
        total_users += num_users
        total_users_comment_parts.append(str(num_users))
        users_vec.append(f'                test_users[..{num_users}].to_vec(), // {num_users} users on {chain["name"]}')
        assets_vec.append(f'                vec![{chain["asset"]}; {num_users}]')
        dst_chain_ids_vec.append(f'                vec![{chain["dst_chain_id"]}; {num_users}]')
        chain_ids_vec.append(chain['chain_id'])

    users_str = 'let users = vec![\n' + ',\n'.join(users_vec) + '\n            ];'
    assets_str = 'let assets = vec![\n' + ',\n'.join(assets_vec) + '\n            ];'
    dst_chain_ids_str = 'let dst_chain_ids = vec![\n' + ',\n'.join(dst_chain_ids_vec) + '\n            ];'
    chain_ids_str = 'let chain_ids = vec![' + ', '.join(chain_ids_vec) + '];'
    total_users_comment = " // " + " + ".join(total_users_comment_parts) + f" = {total_users} total users"
    
    content = re.sub(r'let users = vec!\[.+?\];', users_str, content, flags=re.DOTALL)
    content = re.sub(r'let assets = vec!\[.+?\];', assets_str, content, flags=re.DOTALL)
    content = re.sub(r'let dst_chain_ids = vec!\[.+?\];', dst_chain_ids_str, content, flags=re.DOTALL)
    content = re.sub(r'let chain_ids = vec!\[.+?\];', chain_ids_str, content, flags=re.DOTALL)
    content = re.sub(r'(\s*)\d+, //.*?total users', rf'\g<1>{total_users},{total_users_comment}', content)


    with open('methods/tests/integration_tests.rs', 'w') as f:
        f.write(content)

def update_viewcalls(config):
    with open('malda_rs/src/viewcalls.rs', 'r') as f:
        content = f.read()

    client_params = config['boundless_params']['client_builder']
    
    # --- Client Builder Section ---
    client_builder_pattern = r'(\.config_offer_layer\(\s*\|config\| \{ config(.*)\},\s*\))'
    match = re.search(client_builder_pattern, content, re.DOTALL)
    if match:
        original_block = match.group(1)
        config_block = match.group(2)
        new_config_block = config_block

        # Update prices
        max_price = client_params['max_price_per_cycle_gwei']
        min_price = client_params['min_price_per_cycle_gwei']
        new_config_block = re.sub(r'(\.max_price_per_cycle\(parse_units\(")[^"]+(")', rf'\g<1>{max_price}\g<2>', new_config_block)
        new_config_block = re.sub(r'(\.min_price_per_cycle\(parse_units\(")[^"]+(")', rf'\g<1>{min_price}\g<2>', new_config_block)

        # Update optional params
        for param in ['ramp_up_period', 'lock_timeout', 'timeout']:
            value = client_params.get(param)
            if value is not None:
                new_config_block = re.sub(fr'//\s*(\.{param}\(\d+\))', r'\1', new_config_block)
                new_config_block = re.sub(fr'(\.{param}\()\d+(\))', rf'\g<1>{value}\g<2>', new_config_block)
            else:
                new_config_block = re.sub(fr'^\s*(\.{param}\(\d+\))', r'// \1', new_config_block, flags=re.MULTILINE)

        content = content.replace(original_block, original_block.replace(config_block, new_config_block))

    # --- Request Builder Section ---
    bidding_delay = config['boundless_params']['bidding_start_delay_secs']
    content = re.sub(r'(\.bidding_start\(current_unix_time \+ )\d+(\))', rf'\g<1>{bidding_delay}\g<2>', content)

    with open('malda_rs/src/viewcalls.rs', 'w') as f:
        f.write(content)

def run_test():
    command = "RUST_LOG=info cargo test test_prove_get_proof_data_boundless_load_test -- --nocapture"
    process = subprocess.Popen(command, shell=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    
    for line in iter(process.stdout.readline, b''):
        print(line.decode(), end='')

    process.stdout.close()
    return_code = process.wait()
    
    if return_code:
        raise subprocess.CalledProcessError(return_code, command)

if __name__ == "__main__":
    with open('config.json', 'r') as f:
        config = json.load(f)

    update_integration_tests(config)
    update_viewcalls(config)
    
    print("Configuration updated. Running test...")
    try:
        run_test()
        print("Test completed successfully.")
    except subprocess.CalledProcessError as e:
        print(f"Test failed with exit code {e.returncode}")
