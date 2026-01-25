# Benchmark Datasets Research Report for VSA/Embedding Testing

**Date**: January 25, 2026  
**Purpose**: Comprehensive guide to publicly available benchmark datasets for testing embeddenator VSA operations, retrieval, and file processing.

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Dataset Size Tiers](#dataset-size-tiers)
3. [Text Datasets](#text-datasets)
4. [Document Datasets](#document-datasets)
5. [IR/Retrieval Benchmarks](#irretrieval-benchmarks)
6. [Code Datasets](#code-datasets)
7. [Multi-Format Datasets](#multi-format-datasets)
8. [Kaggle Datasets (No Auth)](#kaggle-datasets-no-auth)
9. [Rust Crates for Dataset Management](#rust-crates-for-dataset-management)
10. [File Format Coverage Matrix](#file-format-coverage-matrix)
11. [Recommended Dataset Stack](#recommended-dataset-stack)

---

## Executive Summary

This report identifies benchmark datasets suitable for testing the embeddenator workspace components:

- **embeddenator-vsa**: Vector encoding, similarity computation, VSA operations
- **embeddenator-retrieval**: Search, indexing, ranking quality
- **embeddenator-fs**: File format parsing, metadata extraction
- **embeddenator-io**: Streaming, chunking, large file handling

### Key Criteria

| Criterion | Requirement |
|-----------|-------------|
| **License** | CC0, MIT, Apache 2.0, Public Domain, CC-BY |
| **Access** | Direct download without authentication |
| **Size Range** | 10MB to 20GB (covering all test tiers) |
| **Formats** | Text, structured data, documents, code |
| **Real-world** | Actual documents, not synthetic data |

---

## Dataset Size Tiers

### Tier 1: Small (< 100MB) - CI/Quick Tests

**Use Case**: Every PR, quick validation, < 1 minute runtime

| Dataset | Size | Format | Source |
|---------|------|--------|--------|
| WikiText-2 | ~12MB | TXT | [HuggingFace](https://huggingface.co/datasets/wikitext) |
| 20 Newsgroups | ~14MB | TXT | [scikit-learn](http://qwone.com/~jason/20Newsgroups/) |
| TREC-COVID (queries) | ~1MB | JSON | [ir-datasets](https://ir-datasets.com/trec-covid.html) |
| arXiv Abstracts Sample | ~50MB | JSON | [Kaggle](https://www.kaggle.com/Cornell-University/arxiv) |
| Enron Emails (subset) | ~40MB | TXT | [CMU](https://www.cs.cmu.edu/~enron/) |

### Tier 2: Medium (100MB - 1GB) - Integration Tests

**Use Case**: Nightly CI, integration tests, 5-15 minute runtime

| Dataset | Size | Format | Source |
|---------|------|--------|--------|
| WikiText-103 | ~500MB | TXT | [HuggingFace](https://huggingface.co/datasets/wikitext) |
| MS MARCO Document (subset) | ~400MB | TSV/JSON | [Microsoft](https://microsoft.github.io/msmarco/) |
| NQ (Natural Questions) | ~800MB | JSONL | [Google](https://ai.google.com/research/NaturalQuestions) |
| AG News | ~120MB | CSV | [Papers With Code](https://paperswithcode.com/dataset/ag-news) |
| DBPedia | ~350MB | CSV | [DBpedia](https://www.dbpedia.org/resources/latest-core/) |

### Tier 3: Large (1GB - 10GB) - Nightly/Weekly Benchmarks

**Use Case**: Weekly performance regression, stress testing

| Dataset | Size | Format | Source |
|---------|------|--------|--------|
| Wikipedia dump (Simple EN) | ~2GB | XML | [Wikimedia](https://dumps.wikimedia.org/simplewiki/) |
| MS MARCO Document (full) | ~3GB | TSV/JSON | [Microsoft](https://microsoft.github.io/msmarco/) |
| The Pile (subset) | ~5GB | JSONL | [EleutherAI](https://pile.eleuther.ai/) |
| RefinedWeb sample | ~2GB | JSONL/Parquet | [HuggingFace](https://huggingface.co/datasets/tiiuae/falcon-refinedweb) |
| C4 (validation split) | ~4GB | JSON | [TensorFlow](https://www.tensorflow.org/datasets/catalog/c4) |

### Tier 4: Full (10GB - 20GB) - Release Validation

**Use Case**: Release candidate validation, full benchmark suite

| Dataset | Size | Format | Source |
|---------|------|--------|--------|
| Wikipedia EN (full dump) | ~20GB | XML/BZ2 | [Wikimedia](https://dumps.wikimedia.org/enwiki/) |
| Common Crawl (single segment) | ~15GB | WARC/WET | [Common Crawl](https://commoncrawl.org/) |
| OpenWebText | ~12GB | TXT | [HuggingFace](https://huggingface.co/datasets/openwebtext) |
| RedPajama (sample) | ~15GB | JSONL | [Together](https://huggingface.co/datasets/togethercomputer/RedPajama-Data-1T-Sample) |

---

## Text Datasets

### 1. WikiText-2 / WikiText-103

**Source**: https://huggingface.co/datasets/wikitext  
**Direct Download**: https://s3.amazonaws.com/research.metamind.io/wikitext/wikitext-103-v1.zip

| Attribute | Value |
|-----------|-------|
| **Size (compressed)** | 181MB (WikiText-103) |
| **Size (uncompressed)** | ~500MB |
| **License** | CC-BY-SA 3.0 |
| **Format** | TXT (UTF-8) |
| **Documents** | ~28,000 articles |
| **Tokens** | ~103M |

**VSA Suitability**:
- ✅ Clean, preprocessed Wikipedia text
- ✅ Good for language modeling benchmarks
- ✅ Consistent formatting (no HTML artifacts)
- ✅ Multiple size variants for different test tiers

```bash
# Download
wget https://s3.amazonaws.com/research.metamind.io/wikitext/wikitext-103-v1.zip
unzip wikitext-103-v1.zip
```

---

### 2. 20 Newsgroups

**Source**: http://qwone.com/~jason/20Newsgroups/  
**Direct Download**: http://qwone.com/~jason/20Newsgroups/20news-bydate.tar.gz

| Attribute | Value |
|-----------|-------|
| **Size (compressed)** | 14MB |
| **Size (uncompressed)** | ~80MB |
| **License** | Public Domain |
| **Format** | TXT (email format) |
| **Documents** | 18,846 posts |
| **Categories** | 20 newsgroups |

**VSA Suitability**:
- ✅ Classic text classification benchmark
- ✅ Natural category structure for clustering tests
- ✅ Email format with headers (metadata extraction testing)
- ✅ Well-documented ground truth labels

---

### 3. Enron Email Dataset

**Source**: https://www.cs.cmu.edu/~enron/  
**Direct Download**: https://www.cs.cmu.edu/~enron/enron_mail_20150507.tar.gz

| Attribute | Value |
|-----------|-------|
| **Size (compressed)** | 423MB |
| **Size (uncompressed)** | ~1.4GB |
| **License** | Public Domain (court release) |
| **Format** | TXT (email format) |
| **Documents** | ~500,000 emails |
| **Users** | 150 mailboxes |

**VSA Suitability**:
- ✅ Real-world enterprise email corpus
- ✅ Hierarchical folder structure
- ✅ Threading relationships (for association testing)
- ✅ Temporal patterns (dated emails)

---

### 4. OpenWebText

**Source**: https://huggingface.co/datasets/openwebtext  
**Direct Download**: https://zenodo.org/record/3834942

| Attribute | Value |
|-----------|-------|
| **Size (compressed)** | ~12GB |
| **Size (uncompressed)** | ~40GB |
| **License** | CC0 1.0 |
| **Format** | TXT/JSONL |
| **Documents** | ~8M web pages |

**VSA Suitability**:
- ✅ Diverse web content
- ✅ Large scale for stress testing
- ✅ Subset available for smaller tests
- ✅ CC0 license (no attribution required)

---

## Document Datasets

### 5. arXiv Dataset

**Source**: https://www.kaggle.com/datasets/Cornell-University/arxiv  
**Alternative**: https://info.arxiv.org/help/bulk_data_s3.html (S3 bucket)

| Attribute | Value |
|-----------|-------|
| **Size (metadata)** | ~3GB JSON |
| **Size (PDFs)** | ~1.1TB (optional) |
| **License** | CC0 (metadata), varies (papers) |
| **Format** | JSON (metadata), PDF (papers), LaTeX (source) |
| **Documents** | ~2.3M papers |

**VSA Suitability**:
- ✅ Structured metadata (title, abstract, authors, categories)
- ✅ Scientific text with technical vocabulary
- ✅ Citation networks for graph-based testing
- ✅ PDF extraction testing with LaTeX ground truth

**Recommended Subset**:
```json
// Download just abstracts (~500MB)
{
  "fields": ["id", "title", "abstract", "categories"],
  "years": [2020, 2021, 2022]
}
```

---

### 6. PubMed Central Open Access

**Source**: https://ftp.ncbi.nlm.nih.gov/pub/pmc/oa_bulk/  
**Direct Download**: FTP access (no auth)

| Attribute | Value |
|-----------|-------|
| **Size** | ~300GB (full), subsets available |
| **License** | CC-BY, CC0 (Open Access subset) |
| **Format** | XML (JATS), PDF |
| **Documents** | ~3.5M articles |

**VSA Suitability**:
- ✅ Medical/scientific domain
- ✅ Structured XML with semantic markup
- ✅ Full-text for long document testing
- ✅ Author affiliations, citations metadata

**Recommended Subset**:
- Commercial Use subset: ~50GB
- Abstracts only: ~2GB

---

### 7. Project Gutenberg

**Source**: https://www.gutenberg.org/  
**Bulk Download**: https://www.gutenberg.org/cache/epub/feeds/

| Attribute | Value |
|-----------|-------|
| **Size** | ~60GB (all), 3GB (English, txt only) |
| **License** | Public Domain |
| **Format** | TXT, HTML, EPUB |
| **Documents** | ~70,000 books |

**VSA Suitability**:
- ✅ Long-form documents (books)
- ✅ Multiple formats per document
- ✅ Historical text variety
- ✅ Clean public domain licensing

**Download Subset**:
```bash
# English books, txt format only (~3GB)
rsync -av --include='*.txt' --exclude='*' \
  aleph.gutenberg.org::gutenberg ./gutenberg-txt/
```

---

## IR/Retrieval Benchmarks

### 8. MS MARCO (Document & Passage)

**Source**: https://microsoft.github.io/msmarco/  
**Direct Download**: Multiple files available

| Variant | Size | Documents | Queries |
|---------|------|-----------|---------|
| **Passage** | 2.9GB | 8.8M passages | 1M |
| **Document** | 22GB | 3.2M documents | 367K |
| **QnA** | 1.1GB | - | 1M |

| Attribute | Value |
|-----------|-------|
| **License** | MIT License |
| **Format** | TSV, JSON |
| **Ground Truth** | Relevance judgments (qrels) |

**VSA Suitability**:
- ✅ **Industry standard** for neural IR evaluation
- ✅ Human-labeled relevance judgments
- ✅ Multiple retrieval tasks (passage, document, QnA)
- ✅ Leaderboard comparability

**Download Commands**:
```bash
# Passage ranking (2.9GB)
wget https://msmarco.z22.web.core.windows.net/msmarcoranking/collection.tar.gz

# Queries
wget https://msmarco.z22.web.core.windows.net/msmarcoranking/queries.tar.gz

# Qrels (ground truth)
wget https://msmarco.z22.web.core.windows.net/msmarcoranking/qrels.train.tsv
```

---

### 9. BEIR Benchmark

**Source**: https://github.com/beir-cellar/beir  
**Datasets**: https://public.ukp.informatik.tu-darmstadt.de/thakur/BEIR/datasets/

| Dataset | Documents | Queries | Size |
|---------|-----------|---------|------|
| **NFCorpus** | 3.6K | 323 | 3MB |
| **SciFact** | 5K | 300 | 5MB |
| **TREC-COVID** | 171K | 50 | 70MB |
| **NQ** | 2.7M | 3.5K | 800MB |
| **HotpotQA** | 5.2M | 7.4K | 1.5GB |
| **FiQA** | 57K | 648 | 30MB |
| **ArguAna** | 8.7K | 1.4K | 10MB |
| **Quora** | 523K | 10K | 50MB |

| Attribute | Value |
|-----------|-------|
| **License** | Various (mostly research-friendly) |
| **Format** | JSONL, TSV |
| **Evaluation** | NDCG@10, Recall@100 |

**VSA Suitability**:
- ✅ **Heterogeneous benchmark** (18 datasets, 9 domains)
- ✅ Zero-shot evaluation (no training data)
- ✅ Standard evaluation scripts
- ✅ Domain diversity (medical, legal, financial, scientific)

**Python Download** (then convert):
```python
from beir import util
dataset = "scifact"
url = f"https://public.ukp.informatik.tu-darmstadt.de/thakur/BEIR/datasets/{dataset}.zip"
util.download_and_unzip(url, "datasets")
```

---

### 10. Natural Questions (NQ)

**Source**: https://ai.google.com/research/NaturalQuestions  
**Direct Download**: https://storage.googleapis.com/natural_questions/

| Attribute | Value |
|-----------|-------|
| **Size** | ~42GB (full), 800MB (simplified) |
| **License** | CC-BY-SA 3.0 |
| **Format** | JSONL |
| **Questions** | 307K (train), 7.8K (dev) |
| **Passages** | Full Wikipedia articles |

**VSA Suitability**:
- ✅ Real Google search queries
- ✅ Long and short answer annotations
- ✅ Wikipedia context (linkable to other datasets)
- ✅ Simplified version for quick testing

---

### 11. TREC Deep Learning Track

**Source**: https://trec.nist.gov/data/deep.html  
**Direct Download**: Via MS MARCO (shared collection)

| Attribute | Value |
|-----------|-------|
| **Size** | Uses MS MARCO collection |
| **License** | TREC license (research use) |
| **Format** | TSV |
| **Queries** | 200 (deeply judged) |
| **Judgments** | ~400K relevance labels |

**VSA Suitability**:
- ✅ **Graded relevance** (0-3 scale, not binary)
- ✅ Deep pooling (many judgments per query)
- ✅ Official TREC evaluation methodology
- ✅ Annual updates (2019-present)

---

## Code Datasets

### 12. The Stack (Subset)

**Source**: https://huggingface.co/datasets/bigcode/the-stack  
**Subset**: https://huggingface.co/datasets/bigcode/the-stack-dedup

| Attribute | Value |
|-----------|-------|
| **Size (full)** | 3TB |
| **Size (subset)** | Configurable (per-language) |
| **License** | Per-file (mostly permissive OSS) |
| **Format** | JSONL, Parquet |
| **Languages** | 358 programming languages |

**VSA Suitability**:
- ✅ Multi-language code corpus
- ✅ Deduplicated version available
- ✅ License metadata per file
- ✅ Repository/file structure preserved

**Per-Language Download**:
```python
from datasets import load_dataset
# Download just Python (~50GB)
ds = load_dataset("bigcode/the-stack", data_dir="data/python", split="train")
```

---

### 13. CodeSearchNet

**Source**: https://github.com/github/CodeSearchNet  
**Direct Download**: https://s3.amazonaws.com/code-search-net/CodeSearchNet/v2/

| Attribute | Value |
|-----------|-------|
| **Size** | ~20GB |
| **License** | MIT (dataset), varies (code) |
| **Format** | JSONL |
| **Languages** | Python, Java, JavaScript, PHP, Ruby, Go |
| **Functions** | ~6M with docstrings |

**VSA Suitability**:
- ✅ Code + natural language (docstrings)
- ✅ Function-level granularity
- ✅ Cross-language search evaluation
- ✅ Standard benchmark for code search

---

### 14. GitHub Code (Sampled)

**Source**: https://huggingface.co/datasets/codeparrot/github-code  

| Attribute | Value |
|-----------|-------|
| **Size** | ~1TB (full), subsets available |
| **License** | Per-file OSS licenses |
| **Format** | Parquet |
| **Languages** | 30+ |

**Recommended Subset**:
- License-filtered: MIT/Apache only
- Size: ~100GB for permissive licenses
- Languages: Python, Rust, JavaScript, TypeScript

---

## Multi-Format Datasets

### 15. Common Crawl

**Source**: https://commoncrawl.org/  
**Direct Download**: https://data.commoncrawl.org/

| Format | Description | Size per segment |
|--------|-------------|------------------|
| **WARC** | Full HTTP response | ~1GB compressed |
| **WET** | Extracted text only | ~150MB compressed |
| **WAT** | Metadata only | ~300MB compressed |

| Attribute | Value |
|-----------|-------|
| **Total Size** | ~400TB per crawl |
| **License** | Terms of Use (free for research) |
| **Frequency** | Monthly crawls |
| **Content Types** | HTML, PDF, images, etc. |

**VSA Suitability**:
- ✅ Real-world web diversity
- ✅ Multiple file formats
- ✅ Metadata for filtering
- ✅ Incremental downloads (single segments)

**Single Segment Download** (~1GB):
```bash
# Get segment list
wget https://data.commoncrawl.org/crawl-data/CC-MAIN-2024-10/wet.paths.gz
gunzip wet.paths.gz

# Download first segment
head -1 wet.paths | xargs -I{} wget https://data.commoncrawl.org/{}
```

---

### 16. OSCAR (Open Super-large Crawled Aggregated corpus)

**Source**: https://oscar-project.org/  
**Download**: https://huggingface.co/datasets/oscar-corpus/OSCAR-2301

| Attribute | Value |
|-----------|-------|
| **Size** | ~8TB (all languages) |
| **License** | CC0 (metadata), content varies |
| **Format** | JSONL, Parquet |
| **Languages** | 150+ |

**Recommended Subset**:
- English unshuffled: ~500GB
- Quality-filtered: ~100GB

---

### 17. Dolma

**Source**: https://huggingface.co/datasets/allenai/dolma  
**Documentation**: https://github.com/allenai/dolma

| Attribute | Value |
|-----------|-------|
| **Size** | ~3TB |
| **License** | ODC-BY (Open Data Commons) |
| **Format** | JSONL |
| **Sources** | Web, books, code, academic |

**VSA Suitability**:
- ✅ Curated, high-quality
- ✅ Source attribution
- ✅ Multiple domains mixed
- ✅ Open license

---

## Kaggle Datasets (No Auth)

While Kaggle typically requires authentication, some datasets have direct download links:

### 18. Amazon Reviews (2018)

**Direct**: https://nijianmo.github.io/amazon/index.html

| Attribute | Value |
|-----------|-------|
| **Size** | ~34GB (full), subsets by category |
| **License** | Research use |
| **Format** | JSON |
| **Reviews** | 233M |

**Subset Sizes**:
- Books: 8.9GB
- Electronics: 2.5GB
- Movies: 3.4GB

---

### 19. Yelp Dataset

**Source**: https://www.yelp.com/dataset  
**Direct Download**: Requires form but no login

| Attribute | Value |
|-----------|-------|
| **Size** | ~9GB |
| **License** | Yelp Dataset License (research) |
| **Format** | JSON |
| **Reviews** | 6.9M |
| **Businesses** | 150K |

---

### 20. IMDB Reviews

**Direct**: https://ai.stanford.edu/~amaas/data/sentiment/

| Attribute | Value |
|-----------|-------|
| **Size** | 80MB |
| **License** | Research use |
| **Format** | TXT |
| **Reviews** | 50K |

---

## Rust Crates for Dataset Management

### Download & HTTP

| Crate | Purpose | Features |
|-------|---------|----------|
| **[reqwest](https://crates.io/crates/reqwest)** | HTTP client | Async, streaming, progress hooks |
| **[ureq](https://crates.io/crates/ureq)** | Sync HTTP | Simple, no async runtime needed |
| **[tokio-util](https://crates.io/crates/tokio-util)** | IO utilities | `ReaderStream` for progress tracking |

### Progress Reporting

| Crate | Purpose | Features |
|-------|---------|----------|
| **[indicatif](https://crates.io/crates/indicatif)** | Progress bars | Multi-bar, templates, ETA |
| **[console](https://crates.io/crates/console)** | Terminal styling | Colors, cursor control |
| **[pbr](https://crates.io/crates/pbr)** | Simple progress | Lightweight alternative |

### Compression & Archives

| Crate | Purpose | Formats |
|-------|---------|---------|
| **[flate2](https://crates.io/crates/flate2)** | GZIP/ZLIB | `.gz`, `.zlib` |
| **[bzip2](https://crates.io/crates/bzip2)** | BZIP2 | `.bz2` |
| **[xz2](https://crates.io/crates/xz2)** | LZMA/XZ | `.xz` |
| **[zstd](https://crates.io/crates/zstd)** | Zstandard | `.zst` (fastest) |
| **[tar](https://crates.io/crates/tar)** | TAR archives | `.tar` |
| **[zip](https://crates.io/crates/zip)** | ZIP archives | `.zip` |
| **[async-compression](https://crates.io/crates/async-compression)** | Async decompression | All formats |

### File Format Parsing

| Crate | Purpose | Formats |
|-------|---------|---------|
| **[serde_json](https://crates.io/crates/serde_json)** | JSON | `.json`, `.jsonl` |
| **[csv](https://crates.io/crates/csv)** | CSV/TSV | `.csv`, `.tsv` |
| **[quick-xml](https://crates.io/crates/quick-xml)** | XML | `.xml` |
| **[roxmltree](https://crates.io/crates/roxmltree)** | XML (DOM) | `.xml` |
| **[pdf-extract](https://crates.io/crates/pdf-extract)** | PDF text | `.pdf` |
| **[lopdf](https://crates.io/crates/lopdf)** | PDF manipulation | `.pdf` |
| **[scraper](https://crates.io/crates/scraper)** | HTML parsing | `.html` |
| **[comrak](https://crates.io/crates/comrak)** | Markdown | `.md` |
| **[pulldown-cmark](https://crates.io/crates/pulldown-cmark)** | Markdown | `.md` |
| **[parquet](https://crates.io/crates/parquet)** | Parquet | `.parquet` |
| **[arrow](https://crates.io/crates/arrow)** | Arrow/IPC | `.arrow` |

### Dataset-Specific Crates

| Crate | Purpose |
|-------|---------|
| **[hf-hub](https://crates.io/crates/hf-hub)** | HuggingFace Hub download |
| **[warc](https://crates.io/crates/warc)** | Common Crawl WARC parsing |
| **[mediawiki-parser](https://crates.io/crates/mediawiki-parser)** | Wikipedia dump parsing |

### Checksum & Integrity

| Crate | Purpose |
|-------|---------|
| **[sha2](https://crates.io/crates/sha2)** | SHA-256 verification |
| **[md-5](https://crates.io/crates/md-5)** | MD5 checksums |
| **[crc32fast](https://crates.io/crates/crc32fast)** | Fast CRC32 |

---

## File Format Coverage Matrix

| Format | Extension | Parsing Crate | Dataset Source |
|--------|-----------|---------------|----------------|
| **Plain Text** | `.txt` | std | WikiText, Gutenberg |
| **Markdown** | `.md` | comrak | GitHub Code |
| **reStructuredText** | `.rst` | Custom parser | Python docs |
| **JSON** | `.json` | serde_json | arXiv, Amazon |
| **JSONL** | `.jsonl` | serde_json (streaming) | The Pile, BEIR |
| **CSV** | `.csv` | csv | AG News, DBPedia |
| **TSV** | `.tsv` | csv | MS MARCO |
| **XML** | `.xml` | quick-xml | Wikipedia, PubMed |
| **HTML** | `.html` | scraper | Common Crawl |
| **PDF** | `.pdf` | pdf-extract | arXiv, PubMed |
| **Parquet** | `.parquet` | parquet | HuggingFace datasets |
| **WARC** | `.warc` | warc | Common Crawl |
| **Source Code** | Various | tree-sitter | The Stack |

---

## Recommended Dataset Stack

### For CI/Quick Tests (< 100MB, < 1 min)

```
embeddenator-testdata-small/
├── wikitext-2/           # 12MB, TXT
├── 20newsgroups/         # 14MB, TXT (emails)
├── beir-scifact/         # 5MB, JSONL
├── beir-nfcorpus/        # 3MB, JSONL
└── imdb-sample/          # 10MB, TXT
```

**Total**: ~50MB compressed

### For Integration Tests (100MB - 1GB, 5-15 min)

```
embeddenator-testdata-medium/
├── wikitext-103/         # 500MB, TXT
├── msmarco-passage/      # 400MB, TSV
├── beir-trec-covid/      # 70MB, JSONL
├── arxiv-abstracts/      # 200MB, JSON
└── codesearchnet-python/ # 300MB, JSONL
```

**Total**: ~1.5GB compressed

### For Nightly Benchmarks (1GB - 10GB)

```
embeddenator-testdata-large/
├── simple-wikipedia/     # 2GB, XML
├── msmarco-document/     # 3GB, TSV
├── the-pile-val/         # 1.5GB, JSONL
├── enron-emails/         # 1.4GB, TXT
└── pubmed-abstracts/     # 2GB, XML
```

**Total**: ~10GB compressed

### For Release Validation (10GB - 20GB)

```
embeddenator-testdata-full/
├── wikipedia-en/         # 20GB, XML
├── openwebtext/          # 12GB, TXT
├── common-crawl-segment/ # 15GB, WARC
└── beir-full/            # 5GB, JSONL
```

**Total**: ~52GB compressed (pick subset)

---

## Download Implementation Example

```rust
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub struct DatasetDownloader {
    client: Client,
    cache_dir: PathBuf,
}

impl DatasetDownloader {
    pub async fn download_with_progress(
        &self,
        url: &str,
        filename: &str,
    ) -> Result<PathBuf, Error> {
        let dest = self.cache_dir.join(filename);
        if dest.exists() {
            return Ok(dest);
        }

        let response = self.client.get(url).send().await?;
        let total_size = response.content_length().unwrap_or(0);

        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .progress_chars("#>-"));

        let mut file = File::create(&dest).await?;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            pb.inc(chunk.len() as u64);
        }

        pb.finish_with_message("Downloaded");
        Ok(dest)
    }
}
```

---

## Checksums & Verification

Always verify downloaded datasets:

| Dataset | Checksum Source |
|---------|-----------------|
| WikiText | HuggingFace metadata |
| MS MARCO | Official README |
| BEIR | `checksums.txt` in repo |
| Common Crawl | Per-segment checksums |

```rust
use sha2::{Sha256, Digest};

pub fn verify_sha256(path: &Path, expected: &str) -> bool {
    let mut file = std::fs::File::open(path).unwrap();
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).unwrap();
    let result = format!("{:x}", hasher.finalize());
    result == expected
}
```

---

## License Summary

| License | Datasets | Commercial Use |
|---------|----------|----------------|
| **CC0** | OpenWebText, WikiText (derived) | ✅ Yes |
| **CC-BY** | Wikipedia, PubMed OA | ✅ Yes (with attribution) |
| **CC-BY-SA** | Wikipedia dumps, NQ | ✅ Yes (share-alike) |
| **MIT** | MS MARCO, CodeSearchNet | ✅ Yes |
| **Apache 2.0** | Many code datasets | ✅ Yes |
| **Public Domain** | 20 Newsgroups, Enron, Gutenberg | ✅ Yes |
| **Research Only** | Amazon Reviews, Yelp | ⚠️ Non-commercial |

---

## Next Steps

1. **Create dataset manifest**: JSON file with URLs, checksums, sizes
2. **Implement downloader**: Async, resumable, with progress
3. **Add to testkit**: `embeddenator-testkit/src/datasets/` module
4. **CI integration**: Download on first run, cache in CI artifacts
5. **Size tier flags**: `--dataset-tier small|medium|large|full`

---

## References

- [BEIR Benchmark](https://github.com/beir-cellar/beir)
- [MS MARCO](https://microsoft.github.io/msmarco/)
- [HuggingFace Datasets](https://huggingface.co/datasets)
- [Papers With Code Datasets](https://paperswithcode.com/datasets)
- [ir-datasets](https://ir-datasets.com/)
- [Common Crawl](https://commoncrawl.org/the-data/get-started/)
