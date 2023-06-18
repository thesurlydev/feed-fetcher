# feed-fetcher

## Features

* Auto-discovery and import of feeds given a website URL; prefix with `https://`
* Import from [OPML](https://en.wikipedia.org/wiki/OPML) file; prefix with `opml!` followed by path or URL
* Import a single feed from a URL; prefix with `feed!` followed by path or URL
* Import your ebooks and use [ChatGPT](https://openai.com/blog/chatgpt) to ask questions about them; prefix with `ebook!` followed by path or URL

## Roadmap

* Import any source supported by [LangChain](https://python.langchain.com/docs/ecosystem/integrations/)
* Percolator 
    * Bubble up new answers and associated source to a ChatGPT prompt as new posts come in.
    * Follow a person and monitor their posts for new answers to your questions.
    * Follow a topic with associated questions and monitor for new answers.
* Return source(s) of answers to questions
* Manage the data used to answer questions
    * Source types and labels
* Investment opportunities as they are discovered 

## Use cases

* Near real-time financial data and company news for investors
* Personalized assistant using your private data
* Highly targeted news for niche interests
* Personalized audit trail of your online activity
* Personalized search engine
* Monitor infrastructure and suggest improvements and cost savings
* Build niche databases for research and analysis

## Daily interactions

* Google Drive
* Google Search
* Google Maps
* Google Calendar
* Google Trends
* Books
* Email
* Slack
* Discord
* Twitter
* Mastodon
* News
* Reddit
* GitHub
* YouTube
* LinkedIn
* Wikipedia



## Components

* Vectorstore
    * FAISS 
* Embeddings
    * HuggingFace
* Model
    * GPT4all 
