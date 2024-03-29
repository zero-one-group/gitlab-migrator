# Zero One Group's GitLab Migrator

This repo contains the source code that [Zero One Group](https://zero-one-group.com/) (ZOG) uses to migrate from GitLab SaaS to GitLab self-managed following [GitLab's announcement](https://about.gitlab.com/blog/2022/03/24/efficient-free-tier/) that the free tier of GitLab SaaS will be limited to five users per namespace.

We use these CLIs to preserve the following in a new self-managed GitLab instance:

- sub-group and project structure;
- users and their avatars;
- specific memberships to sub-groups and projects;
- issue assignees;
- CI variables; and
- pipeline schedules along with the attached CI variables.

Note that we've set things up so that, at the top level, there is only one parent group that contains other sub-groups and projects.

## Manual Setup

Install GitLab on AWS

1. We recommend EC2 with at least 8GiB of RAM. We tried 4GiB, (t3.medium) but we had installation issues. In our case we are using c5.xlarge.
2. Follow the installation manual https://about.gitlab.com/install/
3. Collect a mapping of associated usernames to emails as JSON, and place it in `cache/username_email_mapping.json`. The JSON should contain a single object with usernames as keys and emails as values.

Next, manually export the parent group, and import it to the the target GitLab instance.

## Programmatic Migration

Set up the environment variables by `cp .env.example .env` and replace the environment variables to the appropriate domains and tokens. The source GitLab token must belong to the owner of the parent group, and the target GitLab token must belong to the administrator of the instance.

We then execute the following steps:

1. Download memberships, project archives, issues, pipeline schedules and CI variables, and save it to the `cache/` local directory by running `cargo run download-source-memberships`, `cargo run dowload-source-projects`, `cargo run download-source-ci-variables`, `cargo run download-source-pipeline-schedules`, `cargo run download-source-issues` and `cargo run download-source-project-metadata` respectively. In our case, downloading source projects took a few hours.
2. Add target users based on associated issues and group/project memberships using `cargo run create-target-users`. Rollback (if needed) using `cargo run delete-target-users`.
3. Import target projects by running `cargo run import-target-projects`. Manually create your goups and subgroups. Allow for some time for the projects to be completely imported **after running the import requests**. In our case, it took around 6 hours for all of the project imports to complete. A fast internet connection here helps to avoid timeouts from the server. The client's default timeout is set to 900 seconds. Rollback (if needed) using `cargo run delete-target-projects`. This app is idempotent, so that it's retry tolerant.
4. Add group and project memberships using `cargo run add-target-users-to-groups` and `cargo run add-target-users-to-projects` respectively.
5. Reassign issues to its original assignees using `cargo run reassign-target-issues`. With around 40k issues, this should take about an hour. This app is retry tolerant.
6. Create the project CI variables using `cargo run create-target-ci-variables`.
7. Delete all target pipeline schedules using `cargo run delete-target-pipeline-schedules`, because imported schedules do not come with the CI variables. Re-create the pipeline schedules using `cargo run create-target-pipeline-schedules`.
8. Optionally archive all projects once the new instance is usable using `cargo run archive-source-projects`.

Each app takes into account the **default** rate limits, so it should work right out of the box. With a slow internet connection, it may be necessary [to increase the server's worker timeout](https://docs.gitlab.com/ee/administration/operations/puma.html).

## Finishing Up

What's leftover is:

1. Re-register existing CI runners to the new instance.
2. Rewire existing GitLab integrations by changing the URL and PATs.

---

# Why We Wrote This

## How ZOG Uses GitLab

Every member of ZOG is part of one or more scrum teams, where GitLab's issues, milestones and boards are used as the main workload planning and tracking tool. This applies not only to the engineers and designers, but **everyone** in the company including digital account executives, legal officers, HR admins and sales executives. At the time of writing, ZOG consists of 60 members, of which more than half are not engineers, who use GitLab only for planning purposes (i.e. projects and sub-groups with zero source code). We therefore find ourselves in a rather odd position, where GitLab can be considered to be critical infrastructure for us, but at the same time we have a sizable group of non-power-users. The latter makes it difficult for us to justify paying a per-user subscription fee.

Out of the 60 members, about 25 are part of the technology team, where we use GitLab as a code repository and CI/CD platform on top of the usual planning tools that the rest of the company uses. We use GitLab CI/CD quite heavily, as we run over thousands of jobs every week and use several of our own group-level runners.

Every week, we create close to a thousand issues. Whilst GitLab's boards are excellent for tracking, the UI may be somewhat clunky for creating a large number of issues. For that reason, we have our own internal tool called Krusty that integrates Google Sheets, GitLab and Slack. In particular, one of the most commonly used Krusty apps is a Google Sheet that allows us to create GitLab issues in batch, whilst leveraging Google Sheet's drop-down lists, data validation and auto-fills. So much so that it is commonly known as the 'Krusty Sheet' in the company. Other Krusty integrations include an automated job to look up overdue invoices, create an GitLab issue to follow up the invoice and send a Slack message to notify the finance team.

For ZOG, to migrate out of GitLab means that we need to reinvent a large part of how we operate. For that reason, it makes sense for us to invest a little bit more effort into making GitLab work for us despite the changes to the free-tier GitLab SaaS.

## Problems with Migrating Manually

All of ZOG's GitLab activities live under a single parent group. Under that parent group, there are around 15 sub-groups and close to 150 projects. Whilst there are only 60 ZOG members, there are close to 150 users associated to the parent group, which accounts for ex-members, strategic partners and collaborators from our clients.

It is possible to [migrate projects](https://docs.gitlab.com/ee/user/project/settings/import_export.html) and [migrate groups](https://docs.gitlab.com/ee/user/group/import/), so that it is possible to import these into a new self-managed GitLab instance manually. However, apart from being time consuming and hard to replicate, migrating manually poses a couple infeasibilities:

1. Without public emails (and most users don't have public emails), memberships and assignees are not automatically added. It would be too time consuming and error prone for us to manually add ~300 group/project memberships and ~40k issues.
2. Exported projects lose their CI variables. These secrets can only be re-added manually by project maintainers.
3. Users have to be added manually, and we'll lose everyone's avatars. That just won't fly.

## Other Solutions We Considered

Shout out to [GitLabHost](https://gitlabhost.com/)! If we hadn't had an in-house infrastructure team, we would've gone for GitLabHost's services!
