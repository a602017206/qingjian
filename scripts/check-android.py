import argparse,struct,zipfile

def verify_elf(data,name):
    if len(data)<64 or data[:6]!=b'\x7fELF\x02\x01':raise ValueError(name+'：不是有效的六十四位原生库')
    offset=struct.unpack_from('<Q',data,32)[0]
    size,count=struct.unpack_from('<HH',data,54)
    if size<56 or count==0 or offset+size*count>len(data):raise ValueError(name+'：原生库段表损坏')
    loads=0
    for i in range(count):
        kind,_,file_offset,address,_,file_size,_,alignment=struct.unpack_from('<IIQQQQQQ',data,offset+i*size)
        if kind!=1:continue
        loads+=1
        if file_offset+file_size>len(data):raise ValueError(name+'：原生库内容截断')
        if alignment<16384 or alignment&(alignment-1) or (address-file_offset)%16384:raise ValueError(name+'：原生库不满足 16 KB 对齐，拒绝发布')
    if not loads:raise ValueError(name+'：没有原生加载段')

def verify_apk(path):
    with zipfile.ZipFile(path) as archive:
        if archive.testzip():raise ValueError('安卓安装包压缩内容损坏')
        libs=[name for name in archive.namelist() if name.startswith('lib/') and name.endswith('.so')]
        if not libs:raise ValueError('安卓安装包没有原生库')
        for name in libs:verify_elf(archive.read(name),name)
    return libs

if __name__=='__main__':
    parser=argparse.ArgumentParser(description='校验安卓原生库的十六千字节对齐')
    parser.add_argument('apk');args=parser.parse_args()
    print('安卓原生库对齐检查通过：'+ '、'.join(verify_apk(args.apk)))
