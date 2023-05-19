#!/usr/bin/env python

import os
import sys
import io
import subprocess
from threading import Thread
import time
import random
import pandas as pd
import numpy as np

test_dir = os.path.abspath(os.path.join("..", "client", "test_matrices"))
downloads_dir = os.path.abspath(os.path.join("..", "client", "downloaded_matrices"))

os.makedirs(test_dir, exist_ok=True)
os.makedirs(downloads_dir, exist_ok=True)


sizes = [('tiny', 4), ('small', 12), ('normal', 100), ('big', 1000), ('large', 10000)]
types = ['bool', 'u8', 'u16', 'u32', 'u64', 'i8', 'i16', 'i32', 'i64', 'f64']


def get_file_description_tuple(type: str, size: str, dimension: int):
    file_name = size + '_' + type + '_matrix.csv'

    return (file_name, {
        'file_path': os.path.join(test_dir, file_name),
        'type': type,
        'dimension': dimension,
    })


files = dict([
    get_file_description_tuple(type, size, dimension)
    for type in types for (size, dimension) in sizes
])


def generate_matrices():
    print('Generating test matrices')

    for (file, data) in files.items():
        if os.path.exists(data['file_path']):
            print(f'The file {file} already exists. Skipping')
            continue

        print(f'Generating file {file}')

        if data['type'] == 'f64':
            df = pd.DataFrame([[random.random() * sys.float_info.max
                                for _ in range(data['dimension'])] for _ in range(data['dimension'])])
        else:
            l = 0
            if data['type'] == 'bool':
                r = 1
            else:
                r = 2 ** int(data['type'][1:])
            if data['type'][0] == 'i':
                l = - r // 2
                r = r // 2 - 1

            df = pd.DataFrame([[random.randint(l, r)
                for _ in range(data['dimension'])] for _ in range(data['dimension'])])

        df.to_csv(data['file_path'], header=False, index=False)
  

def handle_client():
    index = handle_client.index
    handle_client.index += 1

    base_path = os.path.join(downloads_dir, str(index))
    os.makedirs(base_path, exist_ok=True)
    for file in os.listdir(base_path):
        os.remove(os.path.join(base_path, file))

    test_results_csv = open(f"{index}_test_results.csv", mode="w")
    test_results_csv.write('dimension,type,time_ns\n')

    def run_client(index, file, data, test_results_csv):
        process = subprocess.Popen(["dotnet", "run", '-c', 'Release', '--no-build'],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, cwd='../client')

        stdin_handle = process.stdin
        stdout_handle = process.stdout

        if stdin_handle == None or stdout_handle == None:
            exit(1)

        print('Client process started successfully')

        def read_to_prompt():
            out = str()

            while True:
                out += stdout_handle.read(1)

                if out.endswith('\n> '):
                    if read_to_prompt.out_prev != out:
                        read_to_prompt.out_prev = out
                        print(out, end='')
                    if not read_to_prompt.three_dots_printed:
                        print("...")
                        read_to_prompt.three_dots_printed = True

                    return out

        read_to_prompt.out_prev = str()
        read_to_prompt.three_dots_printed = False;


        def write_command(cmd: str):
            if write_command.cmd_prev != cmd:
                print(cmd)
                write_command.cmd_prev = cmd

            stdin_handle.write(cmd + '\n')
            stdin_handle.flush()

        write_command.cmd_prev = str()

        read_to_prompt()

        write_command(f'send_data {data["file_path"]}')

        read_to_prompt()

        write_command(f'start_calculation')
        time_start = time.time_ns()

        read_to_prompt()

        sleep_time = 0.1
        while True:
            write_command(f'get_status')
            time_end = time.time_ns()

            out = read_to_prompt()
            if out.find("Calculation complete!") >= 0:
                _, _, rest = out.partition("to file ")
                out_file = rest[:rest.find('.\n')]

                out_file = os.path.join('..', 'client', out_file)
                new_file = os.path.join(downloads_dir, str(index), file)

                os.rename(out_file, new_file)

                print(f'The file has been moved to {new_file}')

                test_results_csv.write(f'{data["dimension"]},{data["type"]},{time_end - time_start}\n')
                test_results_csv.flush()
                break

            time.sleep(sleep_time)
            if sleep_time < 5:
                sleep_time *= 2


        write_command('exit')

        stdin_handle.close()
        stdout_handle.close()

        process.wait()

        exit_code = process.returncode

        print(f'Client process closed with exit code {exit_code}')

    for (file, data) in files.items():
        run_client(index, file, data, test_results_csv)
        
    test_results_csv.close()


handle_client.index = 0


def verify_results():
    client_dirs = [dir for dir in os.listdir(downloads_dir) if dir.isnumeric()]
            
    for client_dir in client_dirs:
        for (file, data) in files.items():
            orig = data['file_path']
            transposed = os.path.join(downloads_dir, client_dir, file)

            print(f'Verifying that {transposed} is the transposed matrix from {orig}')

            df1 = pd.read_csv(orig, header=None)
            df2 = pd.read_csv(transposed, header=None)

            df1np = df1.to_numpy()
            df2np = df2.to_numpy()
            df2np = df2np.transpose()

            assert df1np.shape == df2np.shape
            assert np.array_equal(df1np, df2np)

    print(f'Testing complete!')


generate_matrices()

log_file = open('server.log', 'w')
process = subprocess.Popen(['cargo', 'run', '--release'],
    stdout=subprocess.PIPE, text=True, cwd='../server')

stdout_handle = process.stdout

if stdout_handle == None:
    exit(1)

out = str()
while True:
    out += stdout_handle.read(1)
    if out.find('Listening on port ') != -1 and out.endswith('\n'):
        process.stdout = log_file
        break
    
client_count = 4
threads = [Thread(target=handle_client) for i in range(client_count)]

for thread in threads:
    thread.start()

for thread in threads:
    thread.join()

process.send_signal(15)
process.wait()

log_file.flush()
log_file.close()

verify_results()
