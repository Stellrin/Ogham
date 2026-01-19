## EPUB 结构
EPUB 文件实际上是一个压缩包，里面包含了 HTML 文件、样式表、图片等资源。

book
├── META-INF
│   └── container.xml
├── mimetype
└── OEBPS
    ├── content.opf  // EPUB 主文件，包含元数据、清单和阅读顺序
    ├── toc.ncx      // 目录文件，包含章节信息
    ├── chapter1.html // 章节文件，实际内容
    ├── styles.css   // 样式表，定义 EPUB 的外观
    └── images       // 图片资源

说明：
- `META-INF/container.xml` 指定 EPUB 主文件的位置，通常是 OEBPS/content.opf，因此不同小说这个文件的内容是相同的。
- `mimetype` 指定文件类型为 application/epub+zip，必须是压缩包中的第一个文件且未压缩，内容为单行文本，不同小说这个文件的内容是相同的。
- `OEBPS/content.opf` 包含了小说的元数据（如标题、作者）、清单（列出所有文件）和阅读顺序（指定章节的顺序）。
- `OEBPS/toc.ncx` 定义了章节目录，帮助阅读器导航。(EPUB2)
- `OEBPS/nav.xhtml` 定义了章节目录，帮助阅读器导航。(EPUB3)


## tips
章节的media-type应设置为"application/xhtml+xml"  不要设置为"text/html"，否则会导致部分阅读器无法识别章节内容。



books\zh.Ysgyb.無限淫獄の魔法少女 (二次元ドリームノベルズ) (Japanese Edition)\OEBPS\nav.xhtml
books\zh.Ysgyb.無限淫獄の魔法少女 (二次元ドリームノベルズ) (Japanese Edition)\OEBPS\toc.ncx
books\[きー子]纪子老师作品精选：TS娘系列\OEBPS\nav.xhtml
books\[きー子]纪子老师作品精选：TS娘系列\OEBPS\toc.ncx