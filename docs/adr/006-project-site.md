# Project site

The site is static HTML, CSS and a small release client. Showbiz's neutral aluminium surfaces and separate saturated accent inform its styling; the content is limited to product behavior, requirements and download navigation. The original styling is MIT licensed by Alexander Wanyoike.

GitHub Pages uses the official upload/deploy Actions pattern. Feature PRs target dev and run browser, lint, formatting and build checks. Only main can deploy. Enable the repository's Pages source as GitHub Actions. Promotion requires a release PR from dev to main; feature PRs never deploy or merge automatically.

The release client only constructs download URLs for this repository's known Linux artifact and stable semantic version tags. An offline or empty feed preserves the Releases link. It never claims an unpublished version is downloadable.

Run `yarn verify` and `yarn site:dev`. Browser tests cover desktop, mobile, published-release and offline paths and capture the rendered site.

Reference: https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages
