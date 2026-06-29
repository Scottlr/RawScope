# Non-Goals

RawScope should integrate with data and query ecosystems over time, but its core purpose is large raw-data visual exploration. The following are explicit non-goals.

## Generic Charting Library

RawScope is not a reusable chart toolkit for ordinary application charts. Its rendering model should be driven by large raw datasets, visual aggregation, linked selection, and row evidence.

## React Chart Replacement For Small Data

RawScope is not a replacement for small React charting tools. React and web UI can be useful shells later, but dense data rendering should not be treated as a DOM or small-chart problem.

## BI Dashboard Builder

RawScope is not a BI dashboard system, Tableau clone, Grafana clone, or Plotly clone. BI tools are strongest when metrics and models are already understood; RawScope focuses on the earlier forensic stage.

## SQL Database

RawScope is not a SQL database. Query engines may be integrated later, but the project should not become a storage engine or general SQL execution platform.

## Dataframe Engine

RawScope is not a dataframe library. It may use dataframe-like or columnar concepts internally, but its purpose is visual exploration and evidence, not general tabular computation.

## Notebook Replacement

RawScope is not a notebook replacement. Notebooks remain valuable for flexible analysis and modeling; RawScope should help users decide what deeper notebook analysis is worth doing.

## Cloud Analytics Platform

RawScope is not a cloud analytics platform. The initial product direction is local-first and private dataset exploration.

## General-Purpose UI Framework

RawScope is not a UI framework. UI code should exist to support the workbench and visual analytics workflows, not to become a general component library.

## Full ETL Platform

RawScope is not an extract-transform-load system. File import and data preparation should be limited to what the visual exploration workflow requires.

## Early Plugin System

RawScope should not grow a plugin system before the core data, rendering, selection, and evidence model is stable.
