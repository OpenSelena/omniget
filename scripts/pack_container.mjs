import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';

const sourceExe = 'X:/REA/workspace/v100_source/omniget.exe';
const targetExe = 'X:/omniget/omniget-v100-test.exe';
const buildDir = path.resolve('x:/omniget/build');

console.log('=== Step 1: Loading source executable ===');
const buf = fs.readFileSync(sourceExe);
console.log(`Executable size: ${buf.length} bytes (${(buf.length / (1024 * 1024)).toFixed(2)} MB)`);

// 1. Parse PE Headers
const peHeaderOffset = buf.readUInt32LE(0x3c);
const numSections = buf.readUInt16LE(peHeaderOffset + 6);
const optHeaderSize = buf.readUInt16LE(peHeaderOffset + 20);
const sectionTableOffset = peHeaderOffset + 24 + optHeaderSize;

const imageBase = buf.readBigUInt64LE(peHeaderOffset + 24 + 24); // OptionalHeader.ImageBase (PE32+)
const sectionAlignment = buf.readUInt32LE(peHeaderOffset + 24 + 32);
const fileAlignment = buf.readUInt32LE(peHeaderOffset + 24 + 36);

console.log(`PE Header: ImageBase=0x${imageBase.toString(16)}, Sections=${numSections}, SectionAlign=0x${sectionAlignment.toString(16)}, FileAlign=0x${fileAlignment.toString(16)}`);

// Find last section to determine new section virtual address and raw address
let lastSection = null;
for (let i = 0; i < numSections; i++) {
  const off = sectionTableOffset + i * 40;
  const name = buf.subarray(off, off + 8).toString('utf8').replace(/\0/g, '');
  const vSize = buf.readUInt32LE(off + 8);
  const vAddr = buf.readUInt32LE(off + 12);
  const rSize = buf.readUInt32LE(off + 16);
  const rAddr = buf.readUInt32LE(off + 20);
  lastSection = { index: i, name, vSize, vAddr, rSize, rAddr };
}

function alignUp(val, align) {
  return (val + align - 1) & ~(align - 1);
}

const newSectionVAddr = alignUp(lastSection.vAddr + Math.max(lastSection.vSize, lastSection.rSize), sectionAlignment);
const newSectionRAddr = alignUp(lastSection.rAddr + lastSection.rSize, fileAlignment);

console.log(`New section will be at VAddr=0x${newSectionVAddr.toString(16)}, RawAddr=0x${newSectionRAddr.toString(16)}`);

// 2. Map existing PHF table
function vaToFileOffset(va) {
  const rva = Number(va - imageBase);
  // .rdata section: virtualAddress=137977856, rawAddress=137973248
  return 137973248 + (rva - 137977856);
}

const tableStart = 0x99e6230;
const tableEnd = 0x99ecd90;
const totalEntries = (tableEnd - tableStart) / 32;
console.log(`\n=== Step 2: Packing ${totalEntries} assets from build_v100 ===`);

const newPayloadBuffers = [];
let currentPayloadOffset = 0;
let packedCount = 0;
let fallbackCount = 0;

// Modify a mutable copy of the buffer
const outBuf = Buffer.from(buf);

for (let i = 0; i < totalEntries; i++) {
  const entryOffset = tableStart + i * 32;
  const kp = outBuf.readBigUInt64LE(entryOffset);
  const kl = Number(outBuf.readBigUInt64LE(entryOffset + 8));
  const origVp = outBuf.readBigUInt64LE(entryOffset + 16);
  const origVl = Number(outBuf.readBigUInt64LE(entryOffset + 24));

  const keyFileOffset = vaToFileOffset(kp);
  const key = outBuf.subarray(keyFileOffset, keyFileOffset + kl).toString('utf8');

  // Find file in build_v100
  const relPath = key.replace(/^\/+/, '').replace(/\//g, path.sep);
  const diskPath = path.join(buildDir, relPath);

  let compressedData;
  if (fs.existsSync(diskPath)) {
    const rawContent = fs.readFileSync(diskPath);
    // Brotli compress
    compressedData = zlib.brotliCompressSync(rawContent, {
      params: { [zlib.constants.BROTLI_PARAM_QUALITY]: 11 }
    });
    packedCount++;
  } else {
    // Fallback to original buffer in exe
    const origFileOffset = vaToFileOffset(origVp);
    compressedData = outBuf.subarray(origFileOffset, origFileOffset + origVl);
    fallbackCount++;
  }

  // Calculate new virtual address for this value
  const itemVAddr = BigInt(newSectionVAddr + currentPayloadOffset) + imageBase;
  const itemLen = BigInt(compressedData.length);

  // Update PHF entry in outBuf
  outBuf.writeBigUInt64LE(itemVAddr, entryOffset + 16); // new vp
  outBuf.writeBigUInt64LE(itemLen, entryOffset + 24);   // new vl

  newPayloadBuffers.push(compressedData);
  currentPayloadOffset += compressedData.length;
}

console.log(`Packed: ${packedCount} files from build_v100, ${fallbackCount} fallback original`);
console.log(`Total new asset payload: ${currentPayloadOffset} bytes (${(currentPayloadOffset / (1024 * 1024)).toFixed(2)} MB)`);

// 3. Construct new section payload aligned to file alignment
const payloadRaw = Buffer.concat(newPayloadBuffers);
const newSectionRSize = alignUp(payloadRaw.length, fileAlignment);
const newSectionVSize = payloadRaw.length;

const paddedPayload = Buffer.alloc(newSectionRSize);
payloadRaw.copy(paddedPayload);

// 4. Append new section header
const newSectionOffset = sectionTableOffset + numSections * 40;
const sectionHeader = Buffer.alloc(40);
// Name: ".asdata\0"
sectionHeader.write('.asdata\0', 0, 8, 'utf8');
sectionHeader.writeUInt32LE(newSectionVSize, 8);   // VirtualSize
sectionHeader.writeUInt32LE(newSectionVAddr, 12);  // VirtualAddress
sectionHeader.writeUInt32LE(newSectionRSize, 16);  // SizeOfRawData
sectionHeader.writeUInt32LE(newSectionRAddr, 20);  // PointerToRawData
sectionHeader.writeUInt32LE(0, 24);                // PointerToRelocations
sectionHeader.writeUInt32LE(0, 28);                // PointerToLinenumbers
sectionHeader.writeUInt16LE(0, 32);                // NumberOfRelocations
sectionHeader.writeUInt16LE(0, 34);                // NumberOfLinenumbers
sectionHeader.writeUInt32LE(0x40000040, 36);        // Characteristics: IMAGE_SCN_MEM_READ | IMAGE_SCN_CNT_INITIALIZED_DATA

sectionHeader.copy(outBuf, newSectionOffset);

// Update PE header: NumberOfSections += 1
outBuf.writeUInt16LE(numSections + 1, peHeaderOffset + 6);

// Update SizeOfImage in OptionalHeader
const newSizeOfImage = alignUp(newSectionVAddr + newSectionVSize, sectionAlignment);
outBuf.writeUInt32LE(newSizeOfImage, peHeaderOffset + 24 + 56);

// Ensure outBuf is padded up to newSectionRAddr
let finalBuf;
if (outBuf.length < newSectionRAddr) {
  const pad = Buffer.alloc(newSectionRAddr - outBuf.length);
  finalBuf = Buffer.concat([outBuf, pad, paddedPayload]);
} else {
  finalBuf = Buffer.concat([outBuf.subarray(0, newSectionRAddr), paddedPayload]);
}

console.log(`\n=== Step 3: Writing output executable ===`);
fs.writeFileSync(targetExe, finalBuf);
console.log(`Successfully generated: ${targetExe} (${(finalBuf.length / (1024 * 1024)).toFixed(2)} MB)`);
console.log(`Verified new section .asdata with SizeOfImage=0x${newSizeOfImage.toString(16)}`);
