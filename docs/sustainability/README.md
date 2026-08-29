# Sustainable operation boundary

The auditor runs on demand, stores no server-side data, and avoids scanning
dependency, build, VCS, and vendor trees. Bounded traversal and API pagination
reduce unnecessary work; static contracts and documentation do not require a
resident service.

Release review tracks binary size, dependency growth, compile time, and network
request count as practical proxies. These are not carbon measurements. If usage
becomes material, the project will define a functional unit—one completed audit
of a repository or organization—and measure energy and hardware impact against
that unit before making sustainability claims.
