# Shady spots of language specification 

## Syntax

* Shall comments be supported
* Format of `real` literals

## Types and type conversions

* The `type` alias declaration for predefined types creates some kind of `newtype`, which is incompatible with its carrier in terms of assignemnts. Maybe we need to add something like C++ `to_underlying`
```
type kilometers is real;
type miles is real;

routine kilometers_to_miles(value : kilometers) : miles is 
  ???
end;
```

* The reference types does not have `null` value, which makes them pretty hard to initialise 

* It is said, that `array` should have some kind of `.length` field

## Semantics

* Shall `or` and `and` operators be lazy? *It is in tests now*
* What `==` for reference type stands for? *It is identity check in examples now*

## Runtime
* Shall we implemet garbage collector?