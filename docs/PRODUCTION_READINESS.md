# Production Readiness Checklist

> **Purpose:** This is the "iceberg beneath vibe coding" — the domains, concepts, and questions that need answers before software can be trusted with real users, real data, real money, or real uptime expectations.
>
> Synthesized from: AWS Well-Architected Framework, Google SRE (SLI/SLO/SLA), NIST Secure Software Development Framework (SSDF), OWASP (Web & LLM Top 10), SLSA, OpenTelemetry, Kubernetes production guidance, NIST SP 800-61 Rev. 3 (Incident Response), and DORA's software-delivery metrics.
>
> **How to use this doc:** Copy it into your repo (e.g. `docs/PRODUCTION_READINESS.md`), work through each section with the relevant owner, and check off items as they're answered/implemented. Not every section applies to every project — trim what doesn't apply, but don't skip a section just because it's inconvenient to answer.

---

## Table of Contents

1. [Product, Business, and Launch Reality](#1-product-business-and-launch-reality)
2. [Service Levels, Reliability Targets, and User Promises](#2-service-levels-reliability-targets-and-user-promises)
3. [Architecture and System Design](#3-architecture-and-system-design)
4. [Source Control and Development Workflow](#4-source-control-and-development-workflow)
5. [Local Development, Environments, and Configuration](#5-local-development-environments-and-configuration)
6. [Application Code Quality](#6-application-code-quality)
7. [API and Interface Design](#7-api-and-interface-design)
8. [Data Modelling and Storage](#8-data-modelling-and-storage)
9. [Caching and Performance Layers](#9-caching-and-performance-layers)
10. [Async Processing, Queues, and Event Systems](#10-async-processing-queues-and-event-systems)
11. [Compute, Runtime, and Containerisation](#11-compute-runtime-and-containerisation)
12. [Cloud Infrastructure and Networking](#12-cloud-infrastructure-and-networking)
13. [Infrastructure as Code and Platform Engineering](#13-infrastructure-as-code-and-platform-engineering)
14. [CI/CD, Build, and Release Engineering](#14-cicd-build-and-release-engineering)
15. [Security Engineering](#15-security-engineering)
16. [Software Supply Chain](#16-software-supply-chain)
17. [Privacy, Compliance, and Legal Obligations](#17-privacy-compliance-and-legal-obligations)
18. [Reliability, Failure Handling, and Resilience](#18-reliability-failure-handling-and-resilience)
19. [Performance, Scalability, and Capacity](#19-performance-scalability-and-capacity)
20. [Observability and Monitoring](#20-observability-and-monitoring)
21. [Testing and Verification](#21-testing-and-verification)
22. [Deployment, Migrations, and Rollback Safety](#22-deployment-migrations-and-rollback-safety)
23. [Operations, On-Call, and Incident Response](#23-operations-on-call-and-incident-response)
24. [Backup, Restore, and Disaster Recovery](#24-backup-restore-and-disaster-recovery)
25. [Cost, FinOps, and Capacity Planning](#25-cost-finops-and-capacity-planning)
26. [Multi-Tenancy and Customer Isolation](#26-multi-tenancy-and-customer-isolation)
27. [Admin, Support, and Back-Office Systems](#27-admin-support-and-back-office-systems)
28. [Abuse, Fraud, and Misuse](#28-abuse-fraud-and-misuse)
29. [Notifications, Email, SMS, and Webhooks](#29-notifications-email-sms-and-webhooks)
30. [Data Pipelines, Analytics, and Reporting](#30-data-pipelines-analytics-and-reporting)
31. [AI, ML, and LLM Production Concerns](#31-ai-ml-and-llm-production-concerns)
32. [Mobile, Desktop, and Client-Specific Production Issues](#32-mobile-desktop-and-client-specific-production-issues)
33. [Vendor, Dependency, and Third-Party Risk](#33-vendor-dependency-and-third-party-risk)
34. [Documentation and Knowledge Management](#34-documentation-and-knowledge-management)
35. [Governance, Ownership, and Organizational Reality](#35-governance-ownership-and-organizational-reality)
36. [The "Must-Answer Before Production" Gate](#36-the-must-answer-before-production-gate)
37. [The Biggest Missing "Iceberg" Items](#37-the-biggest-missing-iceberg-items-from-most-vibe-coded-apps)

---

## 1. Product, Business, and Launch Reality

**Concepts:** Product requirements, non-functional requirements, user journeys, critical flows, stakeholder requirements, acceptance criteria, business impact, launch plan, MVP scope, support model, service ownership, internal/external users, customer promises, paid/free plans, usage limits, terms of service, pricing, billing model, onboarding, offboarding, customer support, support SLAs.

- [ ] What problem does this actually solve?
- [ ] Who are the users?
- [ ] What are the critical user journeys?
- [ ] What happens if the system is down for 5 minutes, 1 hour, 1 day?
- [ ] Which workflows are revenue-critical?
- [ ] Which workflows are safety-critical?
- [ ] Which workflows involve personal data, payments, regulated data, or irreversible actions?
- [ ] What is the acceptable error rate?
- [ ] What is the acceptable latency?
- [ ] What is the acceptable data-loss window?
- [ ] Who owns the service after it ships?
- [ ] Who answers support tickets?
- [ ] Who approves launch?
- [ ] Who can pause or roll back the launch?
- [ ] What does "production-ready" mean for this product?

---

## 2. Service Levels, Reliability Targets, and User Promises

**Concepts:** Availability, reliability, durability, SLIs, SLOs, SLAs, error budgets, uptime, latency targets, throughput targets, QPS/RPS, p50/p95/p99 latency, customer-visible errors, maintenance windows, degraded mode, incident severity, support response time.

> Google SRE distinguishes SLIs as quantitative service measurements, SLOs as target values for those measurements, and SLAs as agreements with consequences if targets are missed.

- [ ] What is the service-level objective?
- [ ] Are we promising 99%, 99.9%, 99.99%, or something else?
- [ ] What exact user behavior counts as "available"?
- [ ] What exact user behavior counts as "failed"?
- [ ] What latency percentile matters: p50, p95, p99, or p999?
- [ ] Do internal admin tools need the same SLO as customer-facing flows?
- [ ] What is the error budget?
- [ ] What happens when the error budget is burned?
- [ ] Do we stop feature work to fix reliability?
- [ ] Are planned maintenance windows allowed?
- [ ] Who gets notified when SLOs are breached?
- [ ] Do we have different SLOs for reads, writes, payments, search, uploads, notifications, and background jobs?

---

## 3. Architecture and System Design

**Concepts:** Monolith, modular monolith, microservices, serverless, event-driven architecture, layered/hexagonal/clean architecture, bounded contexts, domain model, service boundaries, dependency graph, data-flow diagram, sequence diagram, ADRs, sync vs async, CAP theorem, consistency model, blast radius, failure domains, single points of failure, dependency inversion, coupling, cohesion, integration patterns.

- [ ] Is this one app, several services, or a distributed system?
- [ ] Where are the service boundaries?
- [ ] What owns each piece of data?
- [ ] Which calls are synchronous?
- [ ] Which work should be asynchronous?
- [ ] What happens if a downstream service is slow?
- [ ] What happens if a downstream service is unavailable?
- [ ] What is the blast radius of a bug?
- [ ] Can one tenant affect another tenant?
- [ ] Can one noisy customer affect everyone else?
- [ ] What are the explicit tradeoffs: simplicity, speed, reliability, cost, flexibility?
- [ ] Is the architecture understandable by a new engineer?
- [ ] Is there a diagram showing runtime components, data flows, and trust boundaries?

---

## 4. Source Control and Development Workflow

**Concepts:** Git, GitHub/GitLab/Bitbucket, branching strategy, trunk-based development, GitFlow, pull requests, code review, protected branches, commit signing, cherry-pick, merge conflict handling, semantic versioning, release tags, changelogs, CODEOWNERS, dependency updates, bot PRs, issue tracking, project boards, feature/hotfix branches.

- [ ] What is the branching strategy?
- [ ] Can anyone push directly to main?
- [ ] Are pull requests required?
- [ ] Who reviews security-sensitive code?
- [ ] Who reviews database migrations?
- [ ] Who reviews infrastructure changes?
- [ ] Do commits need to be signed?
- [ ] How are hotfixes handled?
- [ ] When is cherry-picking acceptable?
- [ ] How do we avoid long-lived branches?
- [ ] How do we know which code version is in production?
- [ ] Can we trace a production deployment back to a commit?

---

## 5. Local Development, Environments, and Configuration

**Concepts:** Local dev, dev/staging/prod, preview/ephemeral environments, test environments, environment parity, environment variables, config files, config schemas, secrets injection, `.env` files, Docker Compose, localstack, seed data, fake services, sandbox APIs, production-like data, synthetic data, environment drift.

- [ ] Can a new engineer run the app locally?
- [ ] Is local setup documented?
- [ ] Are secrets kept out of source control?
- [ ] Are dev, staging, and production meaningfully similar?
- [ ] What differs between staging and prod?
- [ ] Is staging using fake data, sanitized production data, or real customer data?
- [ ] Who can access production config?
- [ ] How are environment variables validated?
- [ ] Can a bad config take production down?
- [ ] Can config changes be rolled back?
- [ ] Are feature flags separated from deploy-time config?

---

## 6. Application Code Quality

**Concepts:** Code structure, modularity, linting, formatting, type checking, static analysis, dependency injection, error handling, exception boundaries, validation, input sanitation, serialization/deserialization, null handling, timeouts, cancellation, retries, concurrency, race conditions, deadlocks, thread safety, memory leaks, timezone/locale handling, numeric precision, floating-point issues, file handling, idempotency, deterministic behavior, graceful shutdown.

- [ ] What inputs are trusted?
- [ ] What inputs are untrusted?
- [ ] Where is validation performed?
- [ ] What happens on partial failure?
- [ ] Are errors handled or swallowed?
- [ ] Are retries safe?
- [ ] Are operations idempotent?
- [ ] Can two requests update the same resource at the same time?
- [ ] Can background jobs conflict with user actions?
- [ ] Are timestamps timezone-safe?
- [ ] Are money values stored safely?
- [ ] Are secrets ever logged?
- [ ] Can the app shut down without corrupting work?
- [ ] Does the app behave predictably after restart?

---

## 7. API and Interface Design

**Concepts:** REST, GraphQL, gRPC, RPC, WebSockets, Server-Sent Events, long/short polling, webhooks, OpenAPI/Swagger, API Gateway, schema validation, request validation, response contracts, pagination, filtering, sorting, idempotency keys, rate limits, quotas, API keys, OAuth, JWT, CORS, content negotiation, versioning, deprecation policy, backwards compatibility, SDKs, client retries, correlation IDs, error codes.

- [ ] Who consumes this API?
- [ ] Is the API public, partner-only, or internal?
- [ ] Is the API contract documented?
- [ ] Are breaking changes allowed?
- [ ] How is versioning handled?
- [ ] What is the error response format?
- [ ] Are errors actionable for clients?
- [ ] Are large responses paginated?
- [ ] Are writes idempotent?
- [ ] Can clients safely retry?
- [ ] Are duplicate webhook deliveries handled?
- [ ] Are webhook signatures verified?
- [ ] How are API keys rotated?
- [ ] What happens when clients use old versions?
- [ ] Are rate limits per user, tenant, IP, token, or endpoint?

---

## 8. Data Modelling and Storage

**Concepts:** Relational DBs (PostgreSQL, MySQL, SQL Server), NoSQL (DynamoDB, MongoDB, Cassandra, Redis, Elasticsearch/OpenSearch), object storage (S3), FTP/SFTP, embedded DB (SQLite), vector DB, data warehouse/lake, schema design, primary/foreign keys, constraints, indexes, query planning, transactions, ACID, isolation levels, deadlocks, optimistic/pessimistic locking, connection pooling, migrations, seed data, backup/restore, PITR, replication, read replicas, sharding, partitioning, hot partitions, archival, retention, deletion, anonymization, encryption at rest.

- [ ] What is the source of truth?
- [ ] What data must never be lost?
- [ ] What data can be regenerated?
- [ ] What data can be cached?
- [ ] What data must be strongly consistent?
- [ ] What data can be eventually consistent?
- [ ] What are the core invariants?
- [ ] Are invariants enforced in the database, app code, or both?
- [ ] Are database migrations reversible?
- [ ] Can migrations run without downtime?
- [ ] What happens if a migration fails halfway through?
- [ ] Are indexes designed for real query patterns?
- [ ] What queries will become slow at 10x traffic?
- [ ] What tables or partitions will become hot?
- [ ] How are large files stored?
- [ ] How are deleted records handled?
- [ ] Can we restore from backup?
- [ ] Have we tested restore, not just backup creation?

---

## 9. Caching and Performance Layers

**Concepts:** Redis, Memcached, CDN cache, browser cache, application/database/query/object cache, write-through, write-behind, cache-aside, cache invalidation, TTLs, stale reads, stale-while-revalidate, cache stampede, cache warming, cache poisoning, distributed cache, edge caching.

- [ ] What is cached?
- [ ] Where is it cached?
- [ ] How long is it cached?
- [ ] Who can invalidate it?
- [ ] Can stale data harm users?
- [ ] Can stale data cause security issues?
- [ ] Can one tenant see another tenant's cached data?
- [ ] What happens when the cache is cold?
- [ ] What happens when Redis is down?
- [ ] Could cache misses overload the database?
- [ ] Do we need cache stampede protection?
- [ ] Are cached responses personalized?
- [ ] Are auth headers respected by CDN caching?

---

## 10. Async Processing, Queues, and Event Systems

**Concepts:** SQS, Kafka, RabbitMQ, Pub/Sub, SNS, event bus, message queues, stream processing, background jobs, workers, cron jobs, scheduled jobs, dead-letter queue, retry queue, poison messages, exponential backoff, jitter, consumer groups, ordering guarantees, duplicate delivery, at-least-once delivery, exactly-once semantics, idempotent consumers, deduplication, transactional outbox, sagas, CQRS, event sourcing, schema registry, event versioning, replay, consumer lag, backpressure.

- [ ] What work happens synchronously?
- [ ] What work happens asynchronously?
- [ ] Are queued jobs idempotent?
- [ ] What happens if a message is delivered twice?
- [ ] What happens if messages arrive out of order?
- [ ] What happens if a worker crashes halfway through?
- [ ] Where do failed messages go?
- [ ] Who reviews the dead-letter queue?
- [ ] Can messages be replayed safely?
- [ ] Is event schema evolution handled?
- [ ] Can old consumers read new events?
- [ ] Can new consumers read old events?
- [ ] How do we detect queue backlog?
- [ ] Can retries create a retry storm?
- [ ] What is the maximum acceptable processing delay?

---

## 11. Compute, Runtime, and Containerisation

**Concepts:** Cloud, VMs, bare metal, containers, Docker, Kubernetes, ECS, Nomad, serverless (Lambda, Cloud Functions), container registry, image tags, image scanning, base images, multi-stage builds, resource requests/limits, CPU/memory limits, autoscaling (HPA), vertical scaling, cold starts, ephemeral filesystems, stateful workloads, sidecars, init containers, cron jobs, graceful shutdown, SIGTERM, readiness/liveness/startup probes.

> Kubernetes uses liveness probes to decide when to restart a container, readiness probes to decide whether it should receive traffic, and startup probes to handle slow-starting containers.

- [ ] Where does the code run?
- [ ] Is the runtime stateful or stateless?
- [ ] What happens when a container restarts?
- [ ] Can the app handle SIGTERM?
- [ ] Does it finish in-flight requests before shutdown?
- [ ] Does it stop accepting traffic before shutdown?
- [ ] Are readiness and liveness checks meaningful?
- [ ] Are resource requests and limits set?
- [ ] What happens when memory is exhausted?
- [ ] What happens when disk fills?
- [ ] Can the service scale horizontally?
- [ ] Are containers running as root?
- [ ] Are images scanned?
- [ ] Are base images patched?
- [ ] How are runtime secrets injected?

---

## 12. Cloud Infrastructure and Networking

**Concepts:** AWS/GCP/Azure, regions, availability zones, VPC/VNet, subnets, route tables, NAT gateway, internet gateway, private endpoints, security groups, network ACLs, DNS, CDN, load balancer, proxy, reverse proxy, firewall, WAF, API gateway, ingress/egress, TLS/SSL certificates, mTLS, certificate rotation, VPN, bastion host, private networking, service discovery, IPv4/IPv6, HTTP/2, HTTP/3, TCP/UDP, WebSockets, gRPC, connection pooling, keep-alive, DNS TTLs.

- [ ] Is the service public or private?
- [ ] What is exposed to the internet?
- [ ] What should never be internet-accessible?
- [ ] Which ports are open?
- [ ] Can services talk to each other only where needed?
- [ ] Is traffic encrypted in transit?
- [ ] Are certificates renewed automatically?
- [ ] What happens if DNS is misconfigured?
- [ ] What happens if the load balancer fails?
- [ ] Are there cross-region dependencies?
- [ ] Are there hidden single points of failure?
- [ ] Is egress controlled?
- [ ] Can a compromised service reach internal databases?
- [ ] Are proxy headers trusted safely?

---

## 13. Infrastructure as Code and Platform Engineering

**Concepts:** Terraform, Pulumi, CloudFormation, AWS CDK, Bicep, Ansible, Helm, Kustomize, ArgoCD, Flux, GitOps, policy as code, OPA, Gatekeeper, Kyverno, service catalog, golden paths, developer portal, scaffolding, reusable modules, state locking, remote state, drift detection, environment promotion, tagging, naming conventions, quotas, namespaces, RBAC, platform APIs.

- [ ] Is infrastructure created manually or through code?
- [ ] Can we recreate production from scratch?
- [ ] Where is Terraform state stored?
- [ ] Is state locked?
- [ ] Who can approve infrastructure changes?
- [ ] Do we detect drift?
- [ ] Are cloud console changes allowed?
- [ ] Are modules versioned?
- [ ] Are environments consistent?
- [ ] Are network, IAM, and database changes reviewed?
- [ ] Are destructive changes protected?
- [ ] Can a bad IaC change delete production data?
- [ ] Are policies enforced automatically?

---

## 14. CI/CD, Build, and Release Engineering

**Concepts:** CI/CD, build/test pipelines, artifact repositories, package registries, container registries, immutable artifacts, build provenance, signed artifacts, SBOM, dependency scanning, deployment pipelines, deployment approvals, release trains, semantic versioning, staging promotion, blue-green deployments, canary releases, rolling deployments, progressive delivery, feature flags, kill switches, rollback, roll-forward, hotfixes, database migration sequencing, expand-contract migrations.

> DORA's current software-delivery metrics are: change lead time, deployment frequency, failed deployment recovery time, change fail rate, and deployment rework rate.

- [ ] What triggers a build?
- [ ] What triggers a deployment?
- [ ] Are builds reproducible?
- [ ] Are artifacts immutable?
- [ ] Can the same artifact be promoted from staging to prod?
- [ ] Are tests required before deployment?
- [ ] Are security scans required before deployment?
- [ ] Are deployments manual, automatic, or gated?
- [ ] How long does deployment take?
- [ ] Can we deploy without downtime?
- [ ] Can we roll back safely?
- [ ] Can we roll back database changes?
- [ ] Can we disable a feature without redeploying?
- [ ] Do we know which deployment introduced a bug?
- [ ] How quickly can a failed deployment be recovered?

---

## 15. Security Engineering

**Concepts:** Authentication, authorization, RBAC, ABAC, ReBAC, IAM, least privilege, OAuth, OpenID Connect, SAML, JWT, sessions, cookies, CSRF, CORS, MFA, password policy, passwordless auth, account recovery, secrets management, key rotation, KMS, HSM, encryption at rest/in transit, mTLS, cryptography, audit logs, security headers, XSS, SQL injection, command injection, SSRF, insecure deserialization, path traversal, file upload scanning, dependency scanning, SAST/DAST/IAST, container scanning, vulnerability management, penetration testing, threat modelling, WAF, DDoS protection, bot protection, supply-chain security, SBOM, code signing, artifact provenance.

> OWASP's Top 10 is a broad-consensus awareness document for critical web application security risks; NIST SSDF provides secure software-development practices intended to reduce vulnerabilities and address their root causes.

- [ ] Who can access the system?
- [ ] Who can access admin features?
- [ ] Who can access customer data?
- [ ] Can users escalate privileges?
- [ ] Is authorization checked server-side?
- [ ] Are object-level permissions enforced?
- [ ] Can one tenant access another tenant's data?
- [ ] Where are secrets stored?
- [ ] How are secrets rotated?
- [ ] Are secrets ever exposed in logs, error messages, frontend bundles, or CI output?
- [ ] Are dependencies scanned?
- [ ] Are containers scanned?
- [ ] Are vulnerabilities triaged?
- [ ] Are uploads scanned?
- [ ] Can uploaded files execute?
- [ ] Are audit logs tamper-resistant?
- [ ] Are security events alerted?
- [ ] Is there a threat model?
- [ ] What would an attacker try first?
- [ ] What is the abuse path?
- [ ] What is the fraud path?
- [ ] What happens if an admin account is compromised?

---

## 16. Software Supply Chain

**Concepts:** Dependency pinning, package lockfiles, private package registries, dependency review, transitive dependencies, dependency confusion, typosquatting, artifact signing, provenance, SLSA, SBOM, build isolation, hermetic/reproducible builds, code signing, container image signing, base image policy, license scanning, open-source compliance, third-party SDKs, GitHub Actions security, CI secrets, branch protection, maintainer access.

> SLSA is a software supply-chain security framework focused on preventing tampering, improving integrity, and securing packages and infrastructure.

- [ ] Do we know every dependency we ship?
- [ ] Do we know every transitive dependency?
- [ ] Are dependencies pinned?
- [ ] Are packages pulled from trusted registries?
- [ ] Can someone publish a malicious package with a similar name?
- [ ] Are builds isolated?
- [ ] Can CI be tampered with?
- [ ] Can a contributor exfiltrate CI secrets?
- [ ] Are artifacts signed?
- [ ] Can production verify artifact provenance?
- [ ] Do we generate an SBOM?
- [ ] Do customers require an SBOM?
- [ ] Are licenses compatible with commercial use?

---

## 17. Privacy, Compliance, and Legal Obligations

**Concepts:** PII, personal/sensitive data, controller, processor, subprocessors, DPA, DPIA, consent, cookie consent, data residency, cross-border transfer, data retention, right to access/deletion, DSAR, GDPR, UK GDPR, CCPA/CPRA, HIPAA, PCI DSS, SOC 2, ISO 27001, audit evidence, records of processing, privacy policy, terms of service, accessibility, WCAG.

> The EDPB defines a controller as the party deciding the purposes and means of processing personal data; PCI DSS provides technical and operational requirements for payment account data; WCAG 2.2 provides recommendations for making web content more accessible.

- [ ] What personal data do we collect?
- [ ] Why do we collect it?
- [ ] Where is it stored?
- [ ] Who can access it?
- [ ] How long do we keep it?
- [ ] Can users export their data?
- [ ] Can users delete their data?
- [ ] Do backups respect deletion requirements?
- [ ] Are subprocessors documented?
- [ ] Do we process payments?
- [ ] Do we store card data, or does a payment provider?
- [ ] Are we subject to PCI DSS?
- [ ] Are we subject to HIPAA, GDPR, SOC 2, ISO 27001, or sector-specific rules?
- [ ] Do we need audit logs?
- [ ] Do we need accessibility conformance?
- [ ] Who signs off on legal/compliance risk?

---

## 18. Reliability, Failure Handling, and Resilience

**Concepts:** High availability, redundancy, failover, disaster recovery, RPO, RTO, backup/restore, retries, exponential backoff, jitter, timeouts, circuit breakers, bulkheads, load shedding, backpressure, graceful degradation, fallback paths, degraded mode, chaos engineering, dependency failure, partial/regional outage, brownout, retry storms, poison messages, split-brain, clock skew, leader election, distributed locks, consensus.

> AWS defines RPO as the maximum acceptable time since the last recovery point; RTO/RPO should be set from business needs when planning disaster recovery.

- [ ] What are the expected failure modes?
- [ ] What happens if the database is slow?
- [ ] What happens if the database is unavailable?
- [ ] What happens if Redis is unavailable?
- [ ] What happens if the queue is backed up?
- [ ] What happens if an external API times out?
- [ ] What happens if email/SMS/payment/auth provider is down?
- [ ] What happens if one region is unavailable?
- [ ] Can the system run in degraded mode?
- [ ] Can non-critical features be disabled?
- [ ] Are retries bounded?
- [ ] Do retries have jitter?
- [ ] Can retries amplify an outage?
- [ ] Are timeouts set everywhere?
- [ ] Are circuit breakers used for fragile dependencies?
- [ ] What is the RPO?
- [ ] What is the RTO?
- [ ] When did we last test restore?

---

## 19. Performance, Scalability, and Capacity

**Concepts:** Optimisation, throughput, QPS/RPS, latency, p95/p99, tail latency, load/stress/soak testing, profiling (CPU, heap), memory leaks, garbage collection, database query planning, indexing, caching, batching, streaming, compression, pagination, async I/O, connection pooling, autoscaling, horizontal/vertical scaling, hot shards/keys, rate limiting, quotas, CDN, edge caching, performance budgets.

- [ ] How many users can the system handle today?
- [ ] How many users should it handle at launch?
- [ ] How many users should it handle at 10x growth?
- [ ] What is the bottleneck?
- [ ] Is the bottleneck CPU, memory, database, network, disk, locks, external APIs, or queue workers?
- [ ] What is the p95 latency?
- [ ] What is the p99 latency?
- [ ] What happens during traffic spikes?
- [ ] Can one customer consume all capacity?
- [ ] Do we have per-tenant quotas?
- [ ] Can the database connection pool be exhausted?
- [ ] Can NAT ports be exhausted?
- [ ] Can logs become the bottleneck?
- [ ] Do we know the maximum safe QPS?
- [ ] Have we tested beyond expected load?

---

## 20. Observability and Monitoring

**Concepts:** Metrics, logs, traces, distributed tracing, OpenTelemetry, structured logging, log aggregation, correlation/request IDs, dashboards, Prometheus, Grafana, ELK, OpenSearch, Datadog, New Relic, Honeycomb, Sentry, error logging, alerting, alert fatigue, SLIs/SLOs, error budgets, synthetic monitoring, uptime checks, black-box/white-box monitoring, audit logs, sampling, cardinality, retention, runbooks.

> OpenTelemetry provides APIs, SDKs, agents, and collectors for telemetry data such as traces, metrics, and logs.

- [ ] Can we tell if the system is healthy?
- [ ] Can we tell if users are affected?
- [ ] Can we trace a request across services?
- [ ] Are logs structured?
- [ ] Do logs include correlation IDs?
- [ ] Can we find all logs for one user action?
- [ ] Are errors grouped?
- [ ] Are alerts actionable?
- [ ] Does every page have a runbook?
- [ ] Do alerts map to SLOs?
- [ ] Are we alerting on symptoms or causes?
- [ ] Do alerts wake someone only when human action is needed?
- [ ] Are dashboards useful during incidents?
- [ ] How long are logs retained?
- [ ] Are sensitive values redacted?
- [ ] Are audit logs separate from debug logs?

---

## 21. Testing and Verification

**Concepts:** Unit, integration, end-to-end, contract, snapshot, smoke, regression, load, stress, soak, chaos, fuzz, property-based, mutation, security, accessibility, browser, mobile, migration, backup-restore, and disaster-recovery tests, canary analysis, synthetic checks, test data management, mocks, stubs, fixtures, sandbox services.

- [ ] What is tested automatically?
- [ ] What is only tested manually?
- [ ] Which tests block deployment?
- [ ] Are tests deterministic?
- [ ] Are flaky tests tracked?
- [ ] Are API contracts tested?
- [ ] Are database migrations tested?
- [ ] Are rollbacks tested?
- [ ] Are permissions tested?
- [ ] Are tenant-isolation rules tested?
- [ ] Are failure modes tested?
- [ ] Are external services mocked realistically?
- [ ] Are load tests run before launch?
- [ ] Are accessibility tests run?
- [ ] Are security tests run?
- [ ] Are backup restores tested?
- [ ] Do tests cover the most critical user journeys?

---

## 22. Deployment, Migrations, and Rollback Safety

**Concepts:** Rolling, blue-green, canary, zero-downtime, and progressive deployments, deployment windows, schema/data migration, expand-contract pattern, backwards-compatible changes, feature flags, kill switches, rollback, roll-forward, hotfix, release notes, deployment freeze, change management, config rollout.

- [ ] Can we deploy during business hours?
- [ ] Can we deploy without downtime?
- [ ] Can old code and new database schema run together?
- [ ] Can new code and old database schema run together?
- [ ] Can migrations be paused?
- [ ] Can migrations be resumed?
- [ ] Can migrations be rolled back?
- [ ] Can data migrations be validated?
- [ ] Can feature flags separate deploy from release?
- [ ] Who can enable a feature?
- [ ] Who can disable a feature?
- [ ] What happens if deployment fails halfway through?
- [ ] Is rollback automatic or manual?
- [ ] How long does rollback take?
- [ ] What customer communication is needed during a bad release?

---

## 23. Operations, On-Call, and Incident Response

**Concepts:** On-call, incident response, escalation policy, severity levels, incident commander, communications lead, runbooks, playbooks, status page, customer/internal comms, support handoff, MTTD, MTTR, postmortems, root cause analysis, corrective actions, incident review, audit trail, pager fatigue, operational readiness review.

> NIST SP 800-61 Rev. 3 integrates incident response into broader cybersecurity risk management; Google SRE postmortems document incident impact, mitigation, root causes, and follow-up actions.

- [ ] Who is on call?
- [ ] What pages them?
- [ ] What should not page them?
- [ ] Who is the backup?
- [ ] Who declares an incident?
- [ ] Who communicates with customers?
- [ ] Who updates the status page?
- [ ] Where are runbooks?
- [ ] Are runbooks tested?
- [ ] What is Severity 1 vs Severity 2 vs Severity 3?
- [ ] How do we escalate to vendors?
- [ ] How do we preserve evidence during a security incident?
- [ ] Who writes the postmortem?
- [ ] Are postmortem actions tracked?
- [ ] Do we fix root causes or just symptoms?

---

## 24. Backup, Restore, and Disaster Recovery

**Concepts:** Backups, snapshots, PITR, restore drills, disaster recovery, business continuity, RPO, RTO, cold/warm/hot standby, multi-AZ, multi-region, data replication, failover/failback, backup encryption, backup access control, immutable backups, ransomware recovery, offsite backups, retention, archive storage.

- [ ] What is backed up?
- [ ] How often?
- [ ] Where are backups stored?
- [ ] Are backups encrypted?
- [ ] Who can access backups?
- [ ] Can backups be deleted by a compromised admin?
- [ ] Are backups isolated from production credentials?
- [ ] How long are backups retained?
- [ ] Can we restore a single customer?
- [ ] Can we restore a single table?
- [ ] Can we restore the full system?
- [ ] How long does restore take?
- [ ] When was the last restore test?
- [ ] Does restored data work with the full application stack?
- [ ] Do we have a runbook for regional disaster?
- [ ] Do we know the business cost of downtime?

---

## 25. Cost, FinOps, and Capacity Planning

**Concepts:** Cloud cost, unit economics, cost per customer/request/tenant, budget alerts, tagging, chargeback, showback, reserved instances, savings plans, autoscaling, rightsizing, storage lifecycle policies, egress costs, observability costs, log volume, trace sampling, API costs, LLM token costs, GPU costs, queue costs, CDN costs, idle resources, quota management, capacity forecasting, sustainability.

> AWS Well-Architected explicitly includes cost optimization and sustainability as architecture pillars, alongside reliability, security, operational excellence, and performance.

- [ ] How much does this cost per month?
- [ ] How much does it cost per active user?
- [ ] How much does one request cost?
- [ ] What happens to cost at 10x usage?
- [ ] Which resources are idle?
- [ ] Are budgets and alerts configured?
- [ ] Are resources tagged?
- [ ] Do logs cost more than compute?
- [ ] Do traces need sampling?
- [ ] Are large objects lifecycle-managed?
- [ ] Are cross-region transfers expensive?
- [ ] Are external APIs billed per request?
- [ ] Are LLM tokens or GPU workloads bounded?
- [ ] Can one abusive customer create a large bill?
- [ ] Who reviews cloud cost weekly?

---

## 26. Multi-Tenancy and Customer Isolation

**Concepts:** Tenant model, tenant ID, row-level security, schema-per-tenant, database-per-tenant, shared/dedicated infrastructure, tenant isolation, noisy-neighbor protection, per-tenant quotas/rate limits, tenant-aware logging/metrics, custom domains, tenant-specific config, tenant deletion, tenant export, tenant billing, tenant admin roles.

- [ ] Is the product single-tenant or multi-tenant?
- [ ] Where is tenant isolation enforced?
- [ ] Can a missing tenant filter leak data?
- [ ] Are permissions tenant-scoped?
- [ ] Are logs tenant-aware?
- [ ] Are metrics tenant-aware?
- [ ] Can one tenant overload shared resources?
- [ ] Do enterprise customers need dedicated infrastructure?
- [ ] Can a tenant be exported?
- [ ] Can a tenant be deleted?
- [ ] Can a tenant have custom retention rules?
- [ ] Can support safely impersonate a tenant user?

---

## 27. Admin, Support, and Back-Office Systems

**Concepts:** Admin panel, support dashboard, customer impersonation, audit logs, RBAC, refunds, billing adjustments, moderation tools, fraud review, user suspension, account recovery, manual overrides, reconciliation, internal notes, data correction, operational tooling, break-glass access.

- [ ] Who can access admin tools?
- [ ] Are admin actions audited?
- [ ] Can support see sensitive data?
- [ ] Can support impersonate users?
- [ ] Is impersonation logged and approved?
- [ ] Can admins delete data?
- [ ] Can admins change billing?
- [ ] Can admins bypass normal validation?
- [ ] Is there break-glass access?
- [ ] How is break-glass access approved?
- [ ] Are internal tools tested and secured like customer-facing apps?
- [ ] What prevents an internal mistake from damaging production?

---

## 28. Abuse, Fraud, and Misuse

**Concepts:** Rate limiting, bot detection, spam prevention, fraud detection, account abuse, credential stuffing, scraping, fake accounts, disposable emails, CAPTCHA, device fingerprinting, IP reputation, velocity checks, anomaly detection, moderation queues, trust and safety, content policy, payment fraud, chargebacks, abuse reporting.

- [ ] How can this be abused?
- [ ] What would a spammer do?
- [ ] What would a scraper do?
- [ ] What would a fraudster do?
- [ ] What would a malicious insider do?
- [ ] Can accounts be created in bulk?
- [ ] Can APIs be scraped?
- [ ] Can free-tier limits be bypassed?
- [ ] Can promo codes be abused?
- [ ] Can invite systems be abused?
- [ ] Can password reset be abused?
- [ ] Do we detect credential stuffing?
- [ ] Do we have manual review tools?
- [ ] Who handles abuse reports?

---

## 29. Notifications, Email, SMS, and Webhooks

**Concepts:** Email/SMS providers, push notifications, in-app notifications, webhooks, retries, delivery status, bounce handling, unsubscribe, preference center, templates, localization, transactional vs marketing email, spam reputation, DKIM, SPF, DMARC, webhook signing, webhook replay protection.

- [ ] Which messages are transactional?
- [ ] Which messages are marketing?
- [ ] Do users have notification preferences?
- [ ] Can users unsubscribe?
- [ ] Are email templates versioned?
- [ ] Are failed sends retried?
- [ ] Are duplicate sends harmful?
- [ ] Can webhook consumers verify authenticity?
- [ ] Can webhook deliveries be replayed?
- [ ] Do webhooks have idempotency IDs?
- [ ] How do we handle provider outages?
- [ ] What happens if SMS costs spike?
- [ ] Are DKIM, SPF, and DMARC configured?

---

## 30. Data Pipelines, Analytics, and Reporting

**Concepts:** Product analytics, event tracking, tracking plan, data warehouse, ETL/ELT, CDC, batch/stream processing, BI dashboards, metrics definitions, data quality, data lineage, data catalog, late-arriving/duplicate events, attribution, A/B testing, experimentation, consent-aware/privacy-preserving analytics.

- [ ] What events are tracked?
- [ ] Are event names and schemas documented?
- [ ] Are analytics events versioned?
- [ ] Can events contain PII?
- [ ] Is user consent respected?
- [ ] How are duplicates handled?
- [ ] How are late events handled?
- [ ] Are business metrics defined consistently?
- [ ] Can finance trust billing reports?
- [ ] Can product trust usage reports?
- [ ] Can support trust account reports?
- [ ] Can dashboards be traced back to source data?

---

## 31. AI, ML, and LLM Production Concerns

**Concepts:** TensorFlow, PyTorch, model serving, batch/online inference, model registry, experiment tracking, feature store, training/evaluation pipeline, model versioning, data/concept drift, model monitoring, shadow deployment, A/B model testing, model rollback, embeddings, vector database, RAG, prompt versioning, prompt injection, insecure output handling, data poisoning, model denial of service, hallucination, grounding, guardrails, human review, evaluation sets, red-teaming, GPU scheduling, token costs, context management, safety filters, fallback models.

> NIST's AI Risk Management Framework helps incorporate trustworthiness considerations into AI system design, development, use, and evaluation; OWASP's LLM Top 10 covers LLM-specific risks such as prompt injection, insecure output handling, training data poisoning, model denial of service, and supply-chain vulnerabilities.

- [ ] Is AI used in a critical decision?
- [ ] Can users be harmed by a wrong output?
- [ ] Is there human review?
- [ ] What data is sent to the model provider?
- [ ] Can prompts leak secrets?
- [ ] Can outputs contain private data?
- [ ] Can users perform prompt injection?
- [ ] Can retrieved documents poison the response?
- [ ] Are model outputs validated before use?
- [ ] Can the model call tools or APIs?
- [ ] Are tool calls permissioned?
- [ ] Are prompts versioned?
- [ ] Are evaluations versioned?
- [ ] Can model behavior be rolled back?
- [ ] How are hallucinations detected?
- [ ] How are token costs controlled?
- [ ] What happens if the model provider is down?
- [ ] Do we need a fallback model?

---

## 32. Mobile, Desktop, and Client-Specific Production Issues

**Concepts:** Mobile app releases, app store review, signed builds, crash reporting, offline mode, sync conflicts, push notifications, deep links, device permissions, secure storage, biometrics, jailbreak/root detection, backwards compatibility, forced upgrades, phased rollout, app version support, desktop auto-update, browser compatibility, accessibility, responsive design.

- [ ] What client versions are supported?
- [ ] Can old clients talk to new APIs?
- [ ] Can new clients talk to old APIs?
- [ ] How are forced upgrades handled?
- [ ] What happens offline?
- [ ] How are sync conflicts resolved?
- [ ] Are tokens stored securely?
- [ ] Are crashes reported?
- [ ] Can push notifications leak sensitive data?
- [ ] Are app builds signed?
- [ ] Can we roll out gradually?
- [ ] Can we halt a bad mobile release?

---

## 33. Vendor, Dependency, and Third-Party Risk

**Concepts:** Cloud provider, auth provider, payment processor, email/SMS provider, analytics provider, CDN, search/map/AI provider, uptime provider, logging provider, support tooling, vendor SLA, vendor status pages, vendor lock-in, exit plan, data export, subprocessors, contract review, procurement, support contracts, escrow, licenses.

- [ ] Which vendors are critical?
- [ ] What happens if each vendor goes down?
- [ ] Do vendors have SLAs?
- [ ] Do vendors process customer data?
- [ ] Are vendors approved subprocessors?
- [ ] Can we export data from each vendor?
- [ ] Can we switch vendors?
- [ ] How long would migration take?
- [ ] Do vendor outages affect our SLA?
- [ ] Do we monitor vendor status?
- [ ] Who owns vendor escalation?
- [ ] Are vendor costs usage-based?
- [ ] Can vendor pricing changes break our margins?

---

## 34. Documentation and Knowledge Management

**Concepts:** README, onboarding docs, architecture diagrams, ADRs, API docs, runbooks, playbooks, data dictionary, service catalog, dependency inventory, owner registry, escalation docs, release docs, threat model, compliance evidence, troubleshooting guides, postmortems, changelogs, deprecation docs.

- [ ] Can a new engineer understand the system?
- [ ] Can someone debug it at 3 a.m.?
- [ ] Where is the architecture documented?
- [ ] Where are service owners documented?
- [ ] Where are runbooks?
- [ ] Where are API contracts?
- [ ] Where are data definitions?
- [ ] Where are known risks?
- [ ] Are docs updated when systems change?
- [ ] Are docs discoverable?
- [ ] Are docs tested?
- [ ] Who owns documentation quality?

---

## 35. Governance, Ownership, and Organizational Reality

**Concepts:** Service ownership, RACI, code owners, architecture review, security review, privacy review, launch review, operational readiness review, change management, risk register, roadmap, tech debt, lifecycle management, deprecation, end-of-life, support ownership, escalation paths, engineering standards.

- [ ] Who owns the service?
- [ ] Who owns the data?
- [ ] Who owns the infrastructure?
- [ ] Who owns security review?
- [ ] Who owns incident response?
- [ ] Who owns customer communication?
- [ ] Who owns cost?
- [ ] Who owns compliance evidence?
- [ ] Who approves risky changes?
- [ ] Who can say "no launch"?
- [ ] What tech debt is accepted?
- [ ] When will accepted risks be revisited?
- [ ] What happens if the original builder leaves?

---

## 36. The "Must-Answer Before Production" Gate

Before shipping, every serious app should have clear answers to these:

- [ ] What can fail?
- [ ] How will we know?
- [ ] Who is alerted?
- [ ] What do they do?
- [ ] How do we recover?
- [ ] How do we roll back?
- [ ] How do we prevent data loss?
- [ ] How do we prevent unauthorized access?
- [ ] How do we protect secrets?
- [ ] How do we handle duplicate requests?
- [ ] How do we handle retries?
- [ ] How do we handle slow dependencies?
- [ ] How do we handle bad deployments?
- [ ] How do we handle schema changes?
- [ ] How do we restore from backup?
- [ ] How do we support users?
- [ ] How do we control cost?
- [ ] How do we prove what happened?
- [ ] Who owns the system after launch?

---

## 37. The Biggest Missing "Iceberg" Items From Most Vibe-Coded Apps

The most commonly invisible but production-critical topics, roughly in order of how often they're skipped:

1. Idempotency
2. Authorization
3. Tenant isolation
4. Schema migrations
5. Backup restore testing
6. Secret management
7. Observability
8. SLOs
9. Incident response
10. Rollback strategy
11. Data retention
12. Supply-chain security
13. Rate limiting
14. Cost controls
15. Abuse prevention
16. Queue failure handling
17. Distributed tracing
18. Threat modeling
19. Compliance evidence
20. Ownership

> **The central production question is not "does the app work on my machine?"**
> **It is: "What happens when everything around the app is slow, duplicated, malicious, expensive, inconsistent, partially failed, or operated by tired humans at 3 a.m.?"**

---

### Sources

- AWS Well-Architected Framework — https://aws.amazon.com/architecture/well-architected/
- Google SRE Book, SLOs — https://sre.google/sre-book/service-level-objectives/
- Kubernetes probes — https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-startup-probes/
- DORA metrics — https://dora.dev/guides/dora-metrics/
- OWASP Top Ten — https://owasp.org/www-project-top-ten/
- SLSA — https://slsa.dev/
- EDPB — https://www.edpb.europa.eu/sme-data-protection-guide/data-controller-data-processor_en
- AWS DR objectives (RPO/RTO) — https://docs.aws.amazon.com/wellarchitected/latest/reliability-pillar/disaster-recovery-dr-objectives.html
- OpenTelemetry — https://opentelemetry.io/
- NIST SP 800-61 Rev. 3 — https://csrc.nist.gov/pubs/sp/800/61/r3/final
- NIST AI Risk Management Framework — https://www.nist.gov/itl/ai-risk-management-framework
